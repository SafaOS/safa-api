use super::{define_syscall, SyscallNum};

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
        /// returns the length of the cwd
        /// if the cwd is too long to fit in `cwd_buf`, the syscall will return [`ErrorStatus::Generic`] (1)
        /// the cwd is currently maximumally 1024 bytes
        sysgetcwd(cwd_buf: Slice<u8>) usize
    }
}

#[inline]
/// Changes the current work dir to `path`
pub fn chdir(path: &str) -> Result<(), ErrorStatus> {
    syschdir(Str::from_str(path)).get()
}

use alloc::string::String;
use alloc::vec::Vec;
use safa_abi::errors::ErrorStatus;
use safa_abi::ffi::slice::Slice;
use safa_abi::ffi::str::Str;

#[inline]
/// Retrieves the current work dir
pub fn getcwd() -> Result<String, ErrorStatus> {
    let mut buffer = [0u8; safa_abi::consts::MAX_PATH_LENGTH];
    let len = sysgetcwd(Slice::from_slice_mut(&mut buffer)).get()?;

    let bytes = Vec::from(&buffer[..len]);
    unsafe { Ok(String::from_utf8_unchecked(bytes)) }
}
