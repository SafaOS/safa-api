use core::time::Duration;

use safa_abi::{
    errors::ErrorStatus,
    ffi::slice::Slice,
    fs::{DirEntry, FileAttr},
    poll::PollEntry,
};

use crate::syscalls::types::{OptionalPtrMut, RequiredPtrMut, Ri};

use super::{define_syscall, SyscallNum};

#[cfg(not(feature = "rustc-dep-of-std"))]
extern crate alloc;

// Directory Iterator related syscalls
define_syscall! {
    SyscallNum::SysFDirIterOpen =>
    {
        /// Opens a directory iterator for the directory with the resource id `dir_ri`
        sysdiriter_open(dir_ri: Ri) Ri
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
    sysdiriter_open(dir_ri).get()
}

#[inline]
/// Gets the next directory entry from a directory iterator,
///
/// see [`sysdiriter_next`] for underlying syscall
pub fn diriter_next(dir_ri: Ri) -> Result<DirEntry, ErrorStatus> {
    let mut dest_direntry: DirEntry = unsafe { core::mem::zeroed() };
    let ptr = RequiredPtrMut::new(&raw mut dest_direntry).into();
    sysdiriter_next(dir_ri, ptr).get().map(|()| dest_direntry)
}

// File related syscalls
define_syscall! {
    SyscallNum::SysIOWrite => {
        /// Writes `len` bytes from `buf` to the file with the resource id `fd` at offset `offset`
        ///
        /// Returns the number of bytes written
        syswrite(fd: Ri, offset: isize, buf: Slice<u8>) usize
    },
    SyscallNum::SysIOTruncate => {
        /// Truncates the file with the resource id `fd` to `len` bytes
        systruncate(fd: Ri, len: usize)
    },
    SyscallNum::SysFSize => {
        /// Gets the size of the file with the resource id `fd` and returns it on success.
        sysfsize(fd: Ri) usize
    },
    SyscallNum::SysFAttrs => {
        /// Gets the file attributes of the file with the resource id `fd` and puts them in `dest_attrs`
        sysfattrs(fd: Ri, dest_attrs: OptionalPtrMut<FileAttr>)
    },
    SyscallNum::SysIORead => {
        /// Reads `len` bytes from the file with the resource id `fd` at offset `offset` into `buf`
        ///
        /// Returns the number of bytes read.
        sysread(fd: Ri, offset: isize, buf: Slice<u8>) usize
    },
    SyscallNum::SysIOSync => {
        /// Syncs the resource with the resource id `fd`
        syssync(ri: Ri)
    },
    SyscallNum::SysIOCommand => {
        /// Sends the command `cmd` to device on the resource `resource` taking an arg `arg`
        sysio_command(ri: Ri, cmd: u16, arg: u64)
    },
    SyscallNum::SysVTTYAlloc => {
        sysvtty_alloc(mother_ri: RequiredPtrMut<Ri>, child_ri: RequiredPtrMut<Ri>, _reserved_zero: usize)
    },
    SyscallNum::SysIOPoll => {
        /// Given a set of resources, waits for any of them to become ready for I/O (with specified events), returns the events that occurred causing the thread to wake up.
        /// # Arguments
        /// * `entries` - A slice of [`PollEntry`] structures, each representing a resource to poll.
        /// * `timeout` - The maximum time to wait for any resource to become ready, in milliseconds, if 0 returns immediately, if u64::MAX waits forever.
        sysiopoll(entries: Slice<PollEntry>, timeout: u64)
    }
}

#[inline]
/// Given a set of resources, waits for any of them to become ready for I/O (with specified events), returns the events that occurred causing the thread to wake up.
/// # Arguments
/// * `entries` - A slice of [`PollEntry`] structures, each representing a resource to poll.
/// * `timeout_ms` - The maximum time to wait for any resource to become ready, in milliseconds, if None or [`Duration::MAX`] waits forever.
pub fn poll_resources(
    entries: &mut [PollEntry],
    timeout_ms: Option<Duration>,
) -> Result<(), ErrorStatus> {
    sysiopoll(
        Slice::from_slice_mut(entries),
        timeout_ms.map(|m| m.as_millis() as u64).unwrap_or(u64::MAX),
    )
    .get()
}

/// Sends the command `cmd` to device on the resource `ri` taking a u64 argument `arg`
pub fn io_command(ri: Ri, cmd: u16, arg: u64) -> Result<(), ErrorStatus> {
    sysio_command(ri, cmd, arg).get()
}

#[inline]
/// Writes `buf.len()` bytes from `buf` to the file with the resource id `fd` at offset `offset`
/// and returns the number of bytes written
pub fn write(fd: Ri, offset: isize, buf: &[u8]) -> Result<usize, ErrorStatus> {
    let slice = Slice::from_slice(buf);
    syswrite(fd, offset, slice).get()
}

#[inline]
/// Truncates the file with the resource id `fd` to `len` bytes
pub fn truncate(fd: Ri, len: usize) -> Result<(), ErrorStatus> {
    systruncate(fd, len).get()
}

#[inline]
/// Gets the size of the file with the resource id `fd`
pub fn fsize(fd: Ri) -> Result<usize, ErrorStatus> {
    sysfsize(fd).get()
}

#[inline]
/// Gets the file attributes of the file with the resource id `fd`
pub fn fattrs(fd: Ri) -> Result<FileAttr, ErrorStatus> {
    let mut attrs: FileAttr = unsafe { core::mem::zeroed() };
    let ptr = RequiredPtrMut::new(&raw mut attrs).into();
    sysfattrs(fd, ptr).get().map(|()| attrs)
}

#[inline]
/// Reads `buf.len()` bytes from the file with the resource id `fd` at offset `offset` into `buf`
pub fn read(fd: Ri, offset: isize, buf: &mut [u8]) -> Result<usize, ErrorStatus> {
    let slice = Slice::from_slice(buf);
    sysread(fd, offset, slice).get()
}

#[inline]
/// Syncs the resource with the resource id `ri`
pub fn sync(ri: Ri) -> Result<(), ErrorStatus> {
    syssync(ri).get()
}

#[inline]
/// Allocates a new VTTY
pub fn vtty_alloc() -> Result<(Ri, Ri), ErrorStatus> {
    let mut mother = 0xAA_AA_AA_AA;
    let mut child = 0xAA_AA_AA_AA;
    unsafe {
        sysvtty_alloc(
            RequiredPtrMut::new_unchecked(&mut mother),
            RequiredPtrMut::new_unchecked(&mut child),
            0,
        )
        .get()
        .map(|()| (mother, child))
    }
}
