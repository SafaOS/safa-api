//! contains functions related to environment variables,
//! api must be initialized before using these functions, see [`super::init`]

use core::{cell::UnsafeCell, ffi::CStr, mem::MaybeUninit, ptr::NonNull};

#[cfg(not(any(feature = "std", feature = "rustc-dep-of-std")))]
extern crate alloc;

#[cfg(feature = "std")]
use std as alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;
use safa_abi::ffi::option::OptZero;
use safa_abi::ffi::slice::Slice;

use alloc::ffi::CString;

use crate::sync::cell::LazyCell;
use crate::sync::locks::Mutex;

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

        // + null + '='
        self.size_hint += key.len() + value.len() + 2;
    }

    #[inline(always)]
    pub fn set(&mut self, key: &[u8], value: &[u8]) {
        for (k, v) in &mut self.env {
            if &**k == key {
                let old_len = v.count_bytes();

                let new_value = CString::new(value)
                    .unwrap_or_else(|_| CStr::from_bytes_until_nul(value).unwrap().into());
                *v = new_value.into_boxed_c_str();
                self.size_hint -= old_len + 1;
                // + null
                self.size_hint += value.len() + 1;
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
    unsafe fn insert_raw(&mut self, raw: &[&[u8]]) {
        self.env.reserve(raw.len());

        for slice in raw {
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

    fn duplicate(&self) -> (Box<[u8]>, Vec<Slice<u8>>) {
        // buf must not reallocate so size_hint must be accurate
        // TODO: maybe rename `size_hint`?
        let mut buf: Vec<u8> = Vec::with_capacity(self.size_hint);
        unsafe { buf.set_len(buf.capacity()) };
        let mut buf = buf.into_boxed_slice();
        let mut offset = 0;

        let mut slices = Vec::with_capacity(self.env.len());

        for (key, value) in &self.env {
            let ptr = unsafe { buf.as_mut_ptr().add(offset) };
            slices.push(unsafe { Slice::from_raw_parts(ptr, key.len() + 1 + value.count_bytes()) });

            buf[offset..offset + key.len()].copy_from_slice(key);
            offset += key.len();

            buf[offset] = b'=';
            offset += 1;

            let value_len = value.count_bytes() + 1;
            buf[offset..offset + value_len].copy_from_slice(value.to_bytes_with_nul());
            offset += value_len;
        }

        (buf, slices)
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct RawEnv {
    args: NonNull<[&'static [u8]]>,
}

impl RawEnv {
    pub const fn new(args: Option<NonNull<[&'static [u8]]>>) -> Self {
        Self {
            args: match args {
                Some(args) => args,
                None => unsafe { NonNull::new_unchecked(&mut []) },
            },
        }
    }

    const unsafe fn into_slice(self) -> &'static [&'static [u8]] {
        unsafe { self.args.as_ref() }
    }
}

pub(super) struct RawEnvStatic(UnsafeCell<MaybeUninit<RawEnv>>);
unsafe impl Sync for RawEnvStatic {}

impl RawEnvStatic {
    pub const fn new() -> Self {
        Self(UnsafeCell::new(MaybeUninit::uninit()))
    }

    pub unsafe fn init(&self, env: RawEnv) {
        unsafe {
            self.0.get().write(MaybeUninit::new(env));
        }
    }

    const unsafe fn get_unchecked(&self) -> &mut RawEnv {
        (*self.0.get()).assume_init_mut()
    }

    pub const unsafe fn as_slice(&self) -> &'static [&'static [u8]] {
        unsafe {
            let raw = self.get_unchecked();
            raw.into_slice()
        }
    }
}

// TODO: refactor all of this
pub(super) static RAW_ENV: RawEnvStatic = RawEnvStatic::new();

// FIXME: use a RwLock
static ENV: LazyCell<Mutex<EnvVars>> = LazyCell::new(|| {
    let mut env = EnvVars::new();
    unsafe { env.insert_raw(RAW_ENV.as_slice()) };
    Mutex::new(env)
});

/// Gets all the environment variables in the current process
#[inline]
pub fn env_get_all() -> Vec<(Box<[u8]>, Box<CStr>)> {
    let env = ENV.lock();
    env.env.clone()
}

#[inline]
pub fn env_get(key: &[u8]) -> Option<Box<[u8]>> {
    let env = ENV.lock();
    env.get(key).map(|v| v.to_vec().into_boxed_slice())
}

#[inline]
pub fn env_set(key: &[u8], value: &[u8]) {
    let mut env = ENV.lock();
    env.set(key, value);
}

#[inline]
pub fn env_remove(key: &[u8]) {
    let mut env = ENV.lock();
    env.remove(key);
}

/// Duplicate the environment variables so that they can be used in a child process by being passed to `_start`.
///
/// # Safety
/// unsafe because it requires for the output to not be dropped before the child process is created.
/// the first element in the tuple represents the raw environment variables, while the second element is a vector of pointers within the first element.
#[inline]
pub(crate) unsafe fn duplicate_env() -> (Box<[u8]>, Vec<Slice<u8>>) {
    let env = ENV.lock();
    env.duplicate()
}

#[inline]
pub fn env_clear() {
    let mut env = ENV.lock();
    env.clear();
}

#[cfg_attr(
    not(any(feature = "std", feature = "rustc-dep-of-std")),
    unsafe(no_mangle)
)]

/// Get an environment variable by key.
///
/// # Safety
/// unsafe because it returns a pointer to the environment variable, which may be invalid after the environment is modified.
pub unsafe extern "C" fn sysenv_get(key: OptZero<Slice<u8>>) -> OptZero<Slice<u8>> {
    unsafe {
        let Some(key) = key.into_option() else {
            return OptZero::none();
        };

        ENV.lock()
            .get(key.as_slice_unchecked())
            .map(|slice| Slice::from_slice(slice))
            .into()
    }
}

#[cfg_attr(
    not(any(feature = "std", feature = "rustc-dep-of-std")),
    unsafe(no_mangle)
)]
/// Set an environment variable by key.
pub extern "C" fn sysenv_set(key: Slice<u8>, value: OptZero<Slice<u8>>) {
    unsafe {
        let key = key
            .try_as_slice()
            .expect("invalid key passed to sysenv_set");

        let value = if let Some(value) = value.into_option() {
            value
                .try_as_slice()
                .expect("invalid value passed to sysenv_set")
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
pub extern "C" fn sysenv_remove(key: Slice<u8>) {
    unsafe {
        let key = key
            .try_as_slice()
            .expect("invalid key passed to sysenv_remove");

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
