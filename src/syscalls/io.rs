use safa_abi::{
    errors::ErrorStatus,
    ffi::slice::Slice,
    fs::{DirEntry, FileAttr},
};

use crate::syscalls::types::{OptionalPtrMut, RequiredPtrMut, Ri};

use super::{define_syscall, err_from_u16, SyscallNum};

#[cfg(not(feature = "rustc-dep-of-std"))]
extern crate alloc;

// Directory Iterator related syscalls
define_syscall! {
    SyscallNum::SysFDirIterOpen =>
    {
        /// Opens a directory iterator for the directory with the resource id `dir_ri`
        sysdiriter_open(dir_ri: Ri, dest_ri: RequiredPtrMut<Ri>)
    },
    SyscallNum::SysDirIterNext => {
        /// Gets the next directory entry from a directory iterator,
        ///
        /// puts the results in `dest_direntry`,
        ///
        /// puts zeroed DirEntry in `dest_direntry` if there are no more entries
        ///
        /// returns [`ErrorStatus::Generic`] (1) if there are no more entries
        sysdiriter_next(dir_ri: Ri, dest_direntry: OptionalPtrMut<DirEntry>)
    }
}

#[inline]
/// Opens a directory iterator for the directory with the resource id `dir_ri`,
/// returns the resource id of the directory iterator
///
/// see [`sysdiriter_open`] for underlying syscall
pub fn diriter_open(dir_ri: Ri) -> Result<Ri, ErrorStatus> {
    let mut dest_fd: usize = 0xAAAAAAAAAAAAAAAAusize;
    let ptr = unsafe { RequiredPtrMut::new_unchecked(&raw mut dest_fd) };
    err_from_u16!(sysdiriter_open(dir_ri, ptr), dest_fd)
}

#[inline]
/// Gets the next directory entry from a directory iterator,
///
/// see [`sysdiriter_next`] for underlying syscall
pub fn diriter_next(dir_ri: Ri) -> Result<DirEntry, ErrorStatus> {
    let mut dest_direntry: DirEntry = unsafe { core::mem::zeroed() };
    let ptr = RequiredPtrMut::new(&raw mut dest_direntry).into();
    err_from_u16!(sysdiriter_next(dir_ri, ptr), dest_direntry)
}

// File related syscalls
define_syscall! {
    SyscallNum::SysIOWrite => {
        /// Writes `len` bytes from `buf` to the file with the resource id `fd` at offset `offset`
        ///
        /// if `dest_wrote` is not null, it will be set to the number of bytes written
        syswrite(fd: Ri, offset: isize, buf: Slice<u8>, dest_wrote: OptionalPtrMut<usize>)
    },
    SyscallNum::SysIOTruncate => {
        /// Truncates the file with the resource id `fd` to `len` bytes
        systruncate(fd: Ri, len: usize)
    },
    SyscallNum::SysFSize => {
        /// Gets the size of the file with the resource id `fd` and puts it in `dest_size`
        sysfsize(fd: Ri, dest_size: OptionalPtrMut<usize>)
    },
    SyscallNum::SysFAttrs => {
        /// Gets the file attributes of the file with the resource id `fd` and puts them in `dest_attrs`
        sysfattrs(fd: Ri, dest_attrs: OptionalPtrMut<FileAttr>)
    },
    SyscallNum::SysIORead => {
        /// Reads `len` bytes from the file with the resource id `fd` at offset `offset` into `buf`
        ///
        /// if `dest_read` is not null, it will be set to the number of bytes read
        sysread(fd: Ri, offset: isize, buf: Slice<u8>, dest_read: OptionalPtrMut<usize>)
    },
    SyscallNum::SysIOSync => {
        /// Syncs the resource with the resource id `fd`
        syssync(ri: Ri)
    },
    SyscallNum::SysIOCommand => {
        /// Sends the command `cmd` to device on the resource `resource` taking an arg `arg`
        sysio_command(ri: Ri, cmd: u16, arg: u64)
    }
}

/// Sends the command `cmd` to device on the resource `ri` taking a u64 argument `arg`
pub fn io_command(ri: Ri, cmd: u16, arg: u64) -> Result<(), ErrorStatus> {
    err_from_u16!(sysio_command(ri, cmd, arg))
}

#[inline]
/// Writes `buf.len()` bytes from `buf` to the file with the resource id `fd` at offset `offset`
/// and returns the number of bytes written
pub fn write(fd: Ri, offset: isize, buf: &[u8]) -> Result<usize, ErrorStatus> {
    let mut dest_wrote = 0;
    let dest_wrote_ptr = RequiredPtrMut::new(&raw mut dest_wrote).into();
    let slice = Slice::from_slice(buf);

    err_from_u16!(syswrite(fd, offset, slice, dest_wrote_ptr), dest_wrote)
}

#[inline]
/// Truncates the file with the resource id `fd` to `len` bytes
pub fn truncate(fd: Ri, len: usize) -> Result<(), ErrorStatus> {
    err_from_u16!(systruncate(fd, len))
}

#[inline]
/// Gets the size of the file with the resource id `fd`
pub fn fsize(fd: Ri) -> Result<usize, ErrorStatus> {
    let mut dest_size = 0;
    let ptr = RequiredPtrMut::new(&raw mut dest_size).into();
    err_from_u16!(sysfsize(fd, ptr), dest_size)
}

#[inline]
/// Gets the file attributes of the file with the resource id `fd`
pub fn fattrs(fd: Ri) -> Result<FileAttr, ErrorStatus> {
    let mut attrs: FileAttr = unsafe { core::mem::zeroed() };
    let ptr = RequiredPtrMut::new(&raw mut attrs).into();
    err_from_u16!(sysfattrs(fd, ptr), attrs)
}

#[inline]
/// Reads `buf.len()` bytes from the file with the resource id `fd` at offset `offset` into `buf`
pub fn read(fd: Ri, offset: isize, buf: &mut [u8]) -> Result<Ri, ErrorStatus> {
    let mut dest_read = 0;
    let dest_read_ptr = RequiredPtrMut::new(&raw mut dest_read).into();
    let slice = Slice::from_slice(buf);
    err_from_u16!(sysread(fd, offset, slice, dest_read_ptr), dest_read)
}

#[inline]
/// Syncs the resource with the resource id `ri`
pub fn sync(ri: Ri) -> Result<(), ErrorStatus> {
    err_from_u16!(syssync(ri))
}
