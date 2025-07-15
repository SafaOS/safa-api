use safa_abi::{
    errors::ErrorStatus,
    raw::io::{DirEntry, OpenOptions},
};

use super::SyscallNum;
use super::{define_syscall, err_from_u16};
use crate::syscalls::types::{OptionalPtrMut, RequiredPtr, RequiredPtrMut, Ri, StrPtr};

define_syscall!(SyscallNum::SysGetDirEntry => {
    /// Gets the directory entry for the path `path` and puts it in `dest_direntry`
    /// path must be valid utf-8
    /// if `dest_direntry` is not null, it will be set to the directory entry
    sysgetdirentry(path_ptr: StrPtr, path_len: usize, dest_direntry: OptionalPtrMut<DirEntry>)
});

#[inline]
pub fn getdirentry(path: &str) -> Result<DirEntry, ErrorStatus> {
    let mut dest_direntry: DirEntry = unsafe { core::mem::zeroed() };
    err_from_u16!(
        sysgetdirentry(path.as_ptr(), path.len(), &raw mut dest_direntry),
        dest_direntry
    )
}

define_syscall! {
    SyscallNum::SysOpenAll => {
        /// Opens the file with the path `path` and puts the resource id in `dest_fd`, with all permissions
        ///
        /// path must be valid utf-8
        sysopen_all(path_ptr: StrPtr, path_len: usize, dest_fd: RequiredPtr<Ri>)
    },
    SyscallNum::SysOpen => {
        /// Opens the file with the path `path` and puts the resource id in `dest_fd`, with a given mode (permissions and flags)
        ///
        /// path must be valid utf-8
        sysopen(path_ptr: StrPtr, path_len: usize, options: OpenOptions, dest_fd: RequiredPtrMut<Ri>)
    },
    SyscallNum::SysCreate => {
        /// Creates the file with the path `path`
        /// path must be valid utf-8
        syscreate_file(path_ptr: StrPtr, path_len: usize)
    },
    SyscallNum::SysCreateDir => {
        /// Creates the directory with the path `path`
        ///
        /// path must be valid utf-8
        syscreate_dir(path_ptr: StrPtr, path_len: usize)
    },
    SyscallNum::SysRemovePath => {
        /// Deletes "removes" a given path
        ///
        /// path must be valid utf-8
        sysremove_path(path_ptr: StrPtr, path_len: usize)
    },
}

#[inline]
/// Opens the path `path` and returns the resource id of the file descriptor, with all permissions
///
/// see [`sysopen_all`] for underlying syscall
pub fn open_all(path: &str) -> Result<Ri, ErrorStatus> {
    let mut dest_fd = 0xAAAAAAAAAAAAAAAAusize;
    err_from_u16!(
        sysopen_all(path.as_ptr(), path.len(), &raw mut dest_fd),
        dest_fd
    )
}

/// Opens the file with the path `path` with a given mode (permissions and flags), returns the resource id of the file descriptor
///
/// see [`sysopen`] for underlying syscall
#[inline]
pub fn open(path: &str, options: OpenOptions) -> Result<Ri, ErrorStatus> {
    let mut dest_fd = 0xAAAAAAAAAAAAAAAAusize;
    err_from_u16!(
        sysopen(path.as_ptr(), path.len(), options, &raw mut dest_fd),
        dest_fd
    )
}

#[inline]
/// Creates the file with the path `path`
///
/// see [`syscreate_file`] for underlying syscall
pub fn create(path: &str) -> Result<(), ErrorStatus> {
    err_from_u16!(syscreate_file(path.as_ptr(), path.len()))
}

#[inline]
/// Creates the directory with the path `path`
///
/// see [`syscreate_dir`] for underlying syscall
pub fn createdir(path: &str) -> Result<(), ErrorStatus> {
    err_from_u16!(syscreate_dir(path.as_ptr(), path.len()))
}

#[inline]
/// Removes a given path
///
/// see [`sysremove_path`] for underlying syscall
pub fn remove_path(path: &str) -> Result<(), ErrorStatus> {
    err_from_u16!(sysremove_path(path.as_ptr(), path.len()))
}
