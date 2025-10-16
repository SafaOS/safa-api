use safa_abi::{
    errors::ErrorStatus,
    ffi::str::Str,
    fs::{DirEntry, OpenOptions},
};

use super::define_syscall;
use super::SyscallNum;
use crate::syscalls::types::{OptionalPtrMut, RequiredPtrMut, Ri};

define_syscall!(SyscallNum::SysFGetDirEntry => {
    /// Gets the directory entry for the path `path` and puts it in `dest_direntry`
    /// path must be valid utf-8
    /// if `dest_direntry` is not null, it will be set to the directory entry
    sysgetdirentry(path: Str, dest_direntry: OptionalPtrMut<DirEntry>)
});

#[inline]
pub fn getdirentry(path: &str) -> Result<DirEntry, ErrorStatus> {
    let mut dest_direntry: DirEntry = unsafe { core::mem::zeroed() };
    let ptr = RequiredPtrMut::new(&raw mut dest_direntry).into();

    sysgetdirentry(Str::from_str(path), ptr)
        .get()
        .map(|()| dest_direntry)
}

define_syscall! {
    SyscallNum::SysFSOpenAll => {
        /// Opens the file with the path `path` and puts the resource id in `dest_fd`, with all permissions
        ///
        /// path must be valid utf-8
        sysopen_all(path: Str) Ri
    },
    SyscallNum::SysFSOpen => {
        /// Opens the file with the path `path` and puts the resource id in `dest_fd`, with a given mode (permissions and flags)
        ///
        /// path must be valid utf-8
        sysopen(path: Str, options: OpenOptions) Ri
    },
    SyscallNum::SysFSCreate => {
        /// Creates the file with the path `path`
        /// path must be valid utf-8
        syscreate_file(path: Str)
    },
    SyscallNum::SysFSCreateDir => {
        /// Creates the directory with the path `path`
        ///
        /// path must be valid utf-8
        syscreate_dir(path: Str)
    },
    SyscallNum::SysFSRemovePath => {
        /// Deletes "removes" a given path
        ///
        /// path must be valid utf-8
        sysremove_path(path: Str)
    },
}

#[inline]
/// Opens the path `path` and returns the resource id of the file descriptor, with all permissions
///
/// see [`sysopen_all`] for underlying syscall
pub fn open_all(path: &str) -> Result<Ri, ErrorStatus> {
    sysopen_all(Str::from_str(path)).get()
}

/// Opens the file with the path `path` with a given mode (permissions and flags), returns the resource id of the file descriptor
///
/// see [`sysopen`] for underlying syscall
#[inline]
pub fn open(path: &str, options: OpenOptions) -> Result<Ri, ErrorStatus> {
    sysopen(Str::from_str(path), options).get()
}

#[inline]
/// Creates the file with the path `path`
///
/// see [`syscreate_file`] for underlying syscall
pub fn create(path: &str) -> Result<(), ErrorStatus> {
    syscreate_file(Str::from_str(path)).get()
}

#[inline]
/// Creates the directory with the path `path`
///
/// see [`syscreate_dir`] for underlying syscall
pub fn createdir(path: &str) -> Result<(), ErrorStatus> {
    syscreate_dir(Str::from_str(path)).get()
}

#[inline]
/// Removes a given path
///
/// see [`sysremove_path`] for underlying syscall
pub fn remove_path(path: &str) -> Result<(), ErrorStatus> {
    sysremove_path(Str::from_str(path)).get()
}
