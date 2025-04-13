//! Module for process-related high-level functions over process related syscalls
//!
//! Such as api initialization functions [`_c_api_init`] and [`sysapi_init`], environment variables, and process arguments

use core::{cell::UnsafeCell, mem::MaybeUninit, ptr::NonNull};

use crate::{
    alloc::GLOBAL_SYSTEM_ALLOCATOR,
    syscalls::{self, define_syscall},
};
use safa_abi::{
    errors::ErrorStatus,
    raw::{processes::TaskMetadata, NonNullSlice, Optional, RawSliceMut},
};

use crate::{syscalls::err_from_u16, syscalls::SyscallNum, Lazy};

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

/// An iterator over the arguments passed to the program.
pub struct ArgsIter {
    args: &'static [NonNullSlice<u8>],
    index: usize,
}

impl ArgsIter {
    pub fn get() -> Self {
        unsafe {
            let args = if let Some(raw) = RAW_ARGS.get_raw() {
                raw.into_slice()
            } else {
                &[]
            };

            Self { args, index: 0 }
        }
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

    pub fn len(&self) -> usize {
        self.args.len()
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

/// Initializes the safa-api
#[cfg_attr(
    not(any(feature = "std", feature = "rustc-dep-of-std")),
    unsafe(no_mangle)
)]
#[inline(always)]
pub extern "C" fn sysapi_init(args: RawSliceMut<NonNullSlice<u8>>) {
    init_args(args);
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
    main: extern "C" fn(argc: i32, argv: *const NonNull<u8>) -> i32,
) -> ! {
    sysapi_init(args);
    let argv_slice = unsafe { args.into_slice_mut().unwrap_or(&mut []) };
    let bytes = args.len() * size_of::<usize>();

    let c_argv_bytes = GLOBAL_SYSTEM_ALLOCATOR.allocate(bytes).unwrap();
    let c_argv_slice = unsafe {
        core::slice::from_raw_parts_mut(c_argv_bytes.as_ptr() as *mut NonNull<u8>, args.len())
    };

    for (i, arg) in argv_slice.iter().enumerate() {
        c_argv_slice[i] = arg.as_non_null();
    }

    let result = main(args.len() as i32, c_argv_slice.as_ptr());
    syscalls::exit(result as usize)
}
