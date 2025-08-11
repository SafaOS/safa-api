use crate::syscalls::types::{OptionalPtrMut, RequiredPtrMut};

use super::{define_syscall, err_from_u16, SyscallNum};

#[cfg(not(feature = "rustc-dep-of-std"))]
extern crate alloc;

define_syscall! {
    SyscallNum::SysPCHDir => {
        /// Changes the current working directory to the path `buf` with length `buf_len`
        /// (expects given buffer to be utf-8)
        syschdir(buf: Str)
    },
    SyscallNum::SysPGetCWD => {
        /// Gets the current working directory and puts it in `cwd_buf` with length `cwd_buf_len`
        /// if `dest_len` is not null, it will be set to the length of the cwd
        /// if the cwd is too long to fit in `cwd_buf`, the syscall will return [`ErrorStatus::Generic`] (1)
        /// the cwd is currently maximumally 1024 bytes
        sysgetcwd(cwd: Slice<u8>, dest_len: OptionalPtrMut<usize>)
    }
}

#[inline]
/// Changes the current work dir to `path`
pub fn chdir(path: &str) -> Result<(), ErrorStatus> {
    err_from_u16!(syschdir(Str::from_str(path)))
}

use alloc::string::String;
use alloc::vec::Vec;
use safa_abi::errors::ErrorStatus;
use safa_abi::ffi::slice::Slice;
use safa_abi::ffi::str::Str;

#[inline]
/// Retrieves the current work dir
pub fn getcwd() -> Result<String, ErrorStatus> {
    let mut buffer = Vec::with_capacity(safa_abi::consts::MAX_PATH_LENGTH);
    unsafe {
        buffer.set_len(buffer.capacity());
    }

    let mut dest_len: usize = 0xAAAAAAAAAAAAAAAAusize;
    let ptr = RequiredPtrMut::new(&raw mut dest_len).into();
    let len = err_from_u16!(sysgetcwd(Slice::from_slice(&buffer), ptr), dest_len)?;

    buffer.truncate(len);
    unsafe { Ok(String::from_utf8_unchecked(buffer)) }
}
