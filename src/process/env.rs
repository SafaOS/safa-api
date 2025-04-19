//! contains functions related to environment variables,
//! api must be initialized before using these functions, see [`super::init`]

use core::{cell::UnsafeCell, ffi::CStr};

#[cfg(not(any(feature = "std", feature = "rustc-dep-of-std")))]
extern crate alloc;

#[cfg(feature = "std")]
use std as alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;
use safa_abi::raw::{NonNullSlice, Optional, RawSlice};

use crate::Lazy;
use alloc::ffi::CString;

use super::args::RawArgsStatic;

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
pub(super) static RAW_ENV: RawArgsStatic = RawArgsStatic::new();

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
