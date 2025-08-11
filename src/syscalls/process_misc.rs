use crate::syscalls::types::{OptionalPtrMut, RequiredPtrMut};

use super::{define_syscall, err_from_u16, SyscallNum};

#[cfg(not(feature = "rustc-dep-of-std"))]
extern crate alloc;

define_syscall! {
    SyscallNum::SysPSbrk => {
        /// Increases the range of the process's data break by `size` bytes and puts the new break pointer in `target_ptr`
        syssbrk(size: isize, target_ptr: OptZero<FFINonNull<*mut u8>>)
    },
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
/// Increases the range of the process's data break by `size` bytes
/// returns the new break pointer
///
/// you should probably use [`crate::alloc::GLOBAL_SYSTEM_ALLOCATOR`] instead for allocating memory
pub fn sbrk(size: isize) -> Result<*mut u8, ErrorStatus> {
    let mut target_ptr: *mut u8 = core::ptr::null_mut();
    let ptr = RequiredPtrMut::new(&raw mut target_ptr).into();
    err_from_u16!(syssbrk(size, ptr), target_ptr)
}

#[inline]
/// Changes the current work dir to `path`
pub fn chdir(path: &str) -> Result<(), ErrorStatus> {
    err_from_u16!(syschdir(Str::from_str(path)))
}

use alloc::string::String;
use alloc::vec::Vec;
use safa_abi::errors::ErrorStatus;
use safa_abi::ffi::option::OptZero;
use safa_abi::ffi::ptr::FFINonNull;
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
