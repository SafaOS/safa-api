//! Module for process-related high-level functions over process related syscalls
//!
//! Such as api initialization functions [`_c_api_init`] and [`sysapi_init`], environment variables, and process arguments
// FIXME: refactor this mess of a module and make it not available when feature = "std" because it breaks things

use core::{cell::UnsafeCell, ffi::CStr, mem::MaybeUninit, ptr::NonNull};

#[cfg(not(any(feature = "std", feature = "rustc-dep-of-std")))]
extern crate alloc;
use crate::{
    alloc::GLOBAL_SYSTEM_ALLOCATOR,
    syscalls::{self, define_syscall},
};
use alloc::boxed::Box;
use alloc::vec::Vec;
use safa_abi::{
    errors::ErrorStatus,
    raw::{processes::TaskMetadata, NonNullSlice, Optional, RawSlice, RawSliceMut},
};

use crate::{syscalls::err_from_u16, syscalls::SyscallNum, Lazy};
use alloc::ffi::CString;
// args

#[derive(Debug, Clone, Copy)]
struct RawArgs {
    args: NonNull<[NonNullSlice<u8>]>,
}

impl RawArgs {
    const fn new(args: NonNull<[NonNullSlice<u8>]>) -> Self {
        Self { args }
    }

    fn len(&self) -> usize {
        unsafe { self.args.as_ref().len() }
    }

    fn get(&self, index: usize) -> Option<NonNullSlice<u8>> {
        unsafe { self.args.as_ref().get(index).copied() }
    }

    unsafe fn into_slice(self) -> &'static [NonNullSlice<u8>] {
        unsafe { self.args.as_ref() }
    }
}

struct RawArgsStatic(UnsafeCell<MaybeUninit<Option<RawArgs>>>);
unsafe impl Sync for RawArgsStatic {}

impl RawArgsStatic {
    const fn new() -> Self {
        Self(UnsafeCell::new(MaybeUninit::uninit()))
    }

    unsafe fn init(&self, args: Option<RawArgs>) {
        unsafe {
            self.0.get().write(MaybeUninit::new(args));
        }
    }

    unsafe fn get(&self, index: usize) -> Option<NonNullSlice<u8>> {
        unsafe { (*self.0.get()).assume_init()?.get(index) }
    }

    unsafe fn len(&self) -> usize {
        if let Some(args) = unsafe { (*self.0.get()).assume_init() } {
            args.len()
        } else {
            0
        }
    }

    unsafe fn get_raw(&self) -> Option<RawArgs> {
        unsafe { (*self.0.get()).assume_init() }
    }

    unsafe fn as_slice(&self) -> &'static [NonNullSlice<u8>] {
        unsafe {
            if let Some(raw) = self.get_raw() {
                raw.into_slice()
            } else {
                &mut []
            }
        }
    }
}

static RAW_ARGS: RawArgsStatic = RawArgsStatic::new();

/// Get the number of arguments passed to the program.
#[cfg_attr(
    not(any(feature = "std", feature = "rustc-dep-of-std")),
    unsafe(no_mangle)
)]
#[inline(always)]
pub extern "C" fn sysget_argc() -> usize {
    unsafe { RAW_ARGS.len() }
}

/// Get the argument at the given index.
#[cfg_attr(
    not(any(feature = "std", feature = "rustc-dep-of-std")),
    unsafe(no_mangle)
)]
#[inline(always)]
pub extern "C" fn sysget_arg(index: usize) -> Optional<NonNullSlice<u8>> {
    unsafe { RAW_ARGS.get(index).into() }
}

// Environment variables

struct EnvVars {
    env: Vec<(Box<[u8]>, Box<CStr>)>,
    /// hints the size of the environment variables in bytes (key.length + value.length + 1 ('='))
    /// which can then be used to duplicate the environment variables
    size_hint: usize,
}

impl EnvVars {
    pub const fn new() -> Self {
        Self {
            env: Vec::new(),
            size_hint: 0,
        }
    }

    pub fn get(&self, key: &[u8]) -> Option<&[u8]> {
        for (k, v) in &self.env {
            if &**k == key {
                return Some(v.to_bytes());
            }
        }
        None
    }

    /// # Safety
    /// This function is unsafe because it should only be used if there is no environment variable with the same key.
    /// otherwise use [`EnvVars::set`]
    #[inline(always)]
    pub unsafe fn push(&mut self, key: &[u8], value: &[u8]) {
        let cstr = CString::new(value)
            .unwrap_or_else(|_| CStr::from_bytes_until_nul(value).unwrap().into());

        self.env
            .push((key.to_vec().into_boxed_slice(), cstr.into_boxed_c_str()));

        self.size_hint += key.len() + value.len() + 1;
    }

    #[inline(always)]
    pub fn set(&mut self, key: &[u8], value: &[u8]) {
        for (k, v) in &mut self.env {
            if &**k == key {
                let old_len = v.count_bytes();

                let new_value = CString::new(value)
                    .unwrap_or_else(|_| CStr::from_bytes_until_nul(value).unwrap().into());
                *v = new_value.into_boxed_c_str();
                self.size_hint -= old_len;
                self.size_hint += value.len();
                return;
            }
        }

        unsafe {
            self.push(key, value);
        }
    }

    #[inline(always)]
    pub fn remove(&mut self, key: &[u8]) {
        for (i, (k, v)) in self.env.iter().enumerate() {
            if &**k == key {
                // order doesn't matter
                self.size_hint -= key.len() + 1 + v.count_bytes();
                self.env.swap_remove(i);
                return;
            }
        }
    }

    /// Insert a raw slice of environment variables into the environment.
    /// # Safety
    /// This function is unsafe because any usage of [`RawSlice<T>`] is unsafe.
    unsafe fn insert_raw(&mut self, raw: &[NonNullSlice<u8>]) {
        self.env.reserve(raw.len());

        for slice in raw {
            let slice = slice.into_slice_mut();
            let mut spilt = slice.splitn(2, |c| *c == b'=');

            let Some(key) = spilt.next() else {
                continue;
            };

            let value = spilt.next();
            let value = value.unwrap_or_default();

            self.push(key, value);
        }
    }

    pub fn clear(&mut self) {
        self.env.clear();
        self.size_hint = 0;
    }

    fn duplicate(&self) -> (Vec<u8>, Vec<RawSlice<u8>>) {
        let mut buf: Vec<u8> = Vec::with_capacity(self.size_hint);
        let mut slices = Vec::with_capacity(self.env.len());

        for (key, value) in &self.env {
            let ptr = unsafe { buf.as_mut_ptr().add(buf.len()) };
            slices.push(unsafe {
                RawSlice::from_raw_parts(ptr, key.len() + 1 + value.count_bytes())
            });

            buf.extend_from_slice(key);
            buf.push(b'=');
            buf.extend_from_slice(value.to_bytes_with_nul());
        }

        (buf, slices)
    }
}

// TODO: refactor all of this
static RAW_ENV: RawArgsStatic = RawArgsStatic::new();

// Lazy always implements Send and Sync LOL
static ENV: Lazy<UnsafeCell<EnvVars>> = Lazy::new(|| {
    let mut env = EnvVars::new();
    unsafe { env.insert_raw(RAW_ENV.as_slice()) };
    UnsafeCell::new(env)
});

// FIXME: unsafe after adding threads
/// Gets all the environment variables in the current process
#[inline]
pub fn env_get_all() -> &'static [(Box<[u8]>, Box<CStr>)] {
    let env = unsafe { &*ENV.get() };
    &env.env
}

#[inline]
pub fn env_get(key: &[u8]) -> Option<&[u8]> {
    let env = unsafe { &*ENV.get() };
    env.get(key)
}

#[inline]
pub fn env_set(key: &[u8], value: &[u8]) {
    let env = unsafe { &mut *ENV.get() };
    env.set(key, value);
}

#[inline]
pub fn env_remove(key: &[u8]) {
    let env = unsafe { &mut *ENV.get() };
    env.remove(key);
}

/// Duplicate the environment variables so that they can be used in a child process by being passed to `_start`.
///
/// # Safety
/// unsafe because it requires for the output to not be dropped before the child process is created.
/// the first element in the tuple represents the raw environment variables, while the second element is a vector of pointers within the first element.
#[inline]
pub(crate) unsafe fn duplicate_env() -> (Vec<u8>, Vec<RawSlice<u8>>) {
    let env = unsafe { &*ENV.get() };
    env.duplicate()
}

#[inline]
pub fn env_clear() {
    let env = unsafe { &mut *ENV.get() };
    env.clear();
}

#[cfg_attr(
    not(any(feature = "std", feature = "rustc-dep-of-std")),
    unsafe(no_mangle)
)]
/// Get an environment variable by key.
pub extern "C" fn sysenv_get(key: RawSlice<u8>) -> Optional<RawSlice<u8>> {
    unsafe {
        let Some(key) = key.into_slice() else {
            return Optional::None;
        };

        env_get(key).map(|slice| RawSlice::from_slice(slice)).into()
    }
}

#[cfg_attr(
    not(any(feature = "std", feature = "rustc-dep-of-std")),
    unsafe(no_mangle)
)]
/// Set an environment variable by key.
pub extern "C" fn sysenv_set(key: RawSlice<u8>, value: RawSlice<u8>) {
    unsafe {
        let Some(key) = key.into_slice() else {
            return;
        };
        let value = if let Some(value) = value.into_slice() {
            value
        } else {
            &[]
        };

        env_set(key, value);
    }
}

#[cfg_attr(
    not(any(feature = "std", feature = "rustc-dep-of-std")),
    unsafe(no_mangle)
)]
/// Remove an environment variable by key.
pub extern "C" fn sysenv_remove(key: RawSlice<u8>) {
    unsafe {
        let Some(key) = key.into_slice() else {
            return;
        };

        env_remove(key);
    }
}

#[cfg_attr(
    not(any(feature = "std", feature = "rustc-dep-of-std")),
    unsafe(no_mangle)
)]
/// Clear all environment variables.
pub extern "C" fn sysenv_clear() {
    env_clear();
}

/// An iterator over the arguments passed to the program.
pub struct ArgsIter {
    args: &'static [NonNullSlice<u8>],
    index: usize,
}

impl ArgsIter {
    pub fn get() -> Self {
        let args = unsafe { RAW_ARGS.as_slice() };
        Self { args, index: 0 }
    }

    pub fn get_index(&self, index: usize) -> Option<NonNullSlice<u8>> {
        self.args.get(index).copied()
    }

    pub fn next(&mut self) -> Option<NonNullSlice<u8>> {
        if self.index < self.args.len() {
            let arg = self.args[self.index];
            self.index += 1;
            Some(arg)
        } else {
            None
        }
    }
    /// The total amount of args in the iterator before calling [`Self::next`]
    pub fn total_len(&self) -> usize {
        self.args.len()
    }
    /// The amount of remaining args in the iterator
    pub fn len(&self) -> usize {
        self.total_len() - self.index
    }
}

static META: Lazy<TaskMetadata> =
    Lazy::new(|| meta_take().expect("failed to take ownership of the task metadata"));

// PEAK design

static STDIN: Lazy<usize> = Lazy::new(|| {
    let stdin: Option<usize> = META.stdin.into();
    if let Some(stdin) = stdin {
        stdin
    } else {
        syscalls::open("dev:/tty").expect("failed to fall back to `dev:/tty` for stdin")
    }
});

static STDOUT: Lazy<usize> = Lazy::new(|| {
    let stdout: Option<usize> = META.stdout.into();
    if let Some(stdout) = stdout {
        stdout
    } else {
        syscalls::open("dev:/tty").expect("failed to fall back to `dev:/tty` for stdout")
    }
});

use syscalls::types::SyscallResult;
static STDERR: Lazy<usize> = Lazy::new(|| {
    let stderr: Option<usize> = META.stderr.into();
    if let Some(stderr) = stderr {
        stderr
    } else {
        syscalls::open("dev:/tty").expect("failed to fall back to `dev:/tty` for stderr")
    }
});

define_syscall!(SyscallNum::SysMetaTake => {
    /// Takes ownership of the task metadata
    /// the task metadata is used to store the stdin, stdout, and stderr file descriptors
    /// this syscall can only be called once otherwise it will return [`ErrorStatus::Generic`] (1)
    sysmeta_take(dest_task: *mut TaskMetadata)
});

#[inline]
pub fn meta_take() -> Result<TaskMetadata, ErrorStatus> {
    let mut dest_meta: TaskMetadata = unsafe { core::mem::zeroed() };
    err_from_u16!(sysmeta_take(&raw mut dest_meta), dest_meta)
}

/// Returns the resource id of the stdout file descriptor
#[cfg_attr(
    not(any(feature = "std", feature = "rustc-dep-of-std")),
    unsafe(no_mangle)
)]
#[inline(always)]
pub extern "C" fn sysmeta_stdout() -> usize {
    **STDOUT
}

/// Returns the resource id of the stderr file descriptor
#[cfg_attr(
    not(any(feature = "std", feature = "rustc-dep-of-std")),
    unsafe(no_mangle)
)]
#[inline(always)]
pub extern "C" fn sysmeta_stderr() -> usize {
    **STDERR
}

/// Returns the resource id of the stdin file descriptor
#[cfg_attr(
    not(any(feature = "std", feature = "rustc-dep-of-std")),
    unsafe(no_mangle)
)]
#[inline(always)]
pub extern "C" fn sysmeta_stdin() -> usize {
    **STDIN
}

// Initialization

fn init_args(args: RawSliceMut<NonNullSlice<u8>>) {
    unsafe {
        let slice = args
            .into_slice_mut()
            .map(|inner| RawArgs::new(NonNull::new_unchecked(inner as *mut _)));
        RAW_ARGS.init(slice)
    }
}

fn init_env(env: RawSliceMut<NonNullSlice<u8>>) {
    unsafe {
        let slice = env
            .into_slice_mut()
            .map(|inner| RawArgs::new(NonNull::new_unchecked(inner as *mut _)));
        RAW_ENV.init(slice)
    }
}

/// Initializes the safa-api
#[cfg_attr(
    not(any(feature = "std", feature = "rustc-dep-of-std")),
    unsafe(no_mangle)
)]
#[inline(always)]
pub extern "C" fn sysapi_init(
    args: RawSliceMut<NonNullSlice<u8>>,
    env: RawSliceMut<NonNullSlice<u8>>,
) {
    init_args(args);
    init_env(env);
}

/// Initializes the safa-api, converts arguments to C-style arguments, calls `main`, and exits with the result
/// main are designed as C main function,
///
/// this function is designed to be called from C code at _start before main,
/// main should be passed as a parameter
#[cfg_attr(
    not(any(feature = "std", feature = "rustc-dep-of-std")),
    unsafe(no_mangle)
)]
pub unsafe extern "C" fn _c_api_init(
    args: RawSliceMut<NonNullSlice<u8>>,
    env: RawSliceMut<NonNullSlice<u8>>,
    main: extern "C" fn(argc: i32, argv: *const *const u8) -> i32,
) -> ! {
    sysapi_init(args, env);

    // Convert SafaOS `_start` arguments to `main` arguments
    fn c_main_args(args: RawSliceMut<NonNullSlice<u8>>) -> (i32, *const *const u8) {
        let argv_slice = unsafe { args.into_slice_mut().unwrap_or_default() };
        if argv_slice.is_empty() {
            return (0, core::ptr::null());
        }

        let bytes = (args.len() + 1) * size_of::<usize>();

        let c_argv_bytes = GLOBAL_SYSTEM_ALLOCATOR.allocate(bytes).unwrap();
        let c_argv_slice = unsafe {
            core::slice::from_raw_parts_mut(c_argv_bytes.as_ptr() as *mut *const u8, args.len() + 1)
        };

        for (i, arg) in argv_slice.iter().enumerate() {
            c_argv_slice[i] = arg.as_ptr();
        }

        c_argv_slice[args.len()] = core::ptr::null();
        (args.len() as i32, c_argv_slice.as_ptr())
    }

    let (argc, argv) = c_main_args(args);
    let result = main(argc, argv);
    syscalls::exit(result as usize)
}
