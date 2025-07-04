//! This module defines exposes raw SafaOS syscalls
//! and their rust counterparts

#[cfg(not(feature = "rustc-dep-of-std"))]
extern crate alloc;

use super::types::{
    OptionalPtrMut, OptionalStrPtr, Pid, RequiredPtr, RequiredPtrMut, Ri, StrPtr, StrPtrMut,
    SyscallResult,
};
use super::SyscallNum;
use super::{define_syscall, DirEntry, FileAttr, RawSlice, RawSliceMut};
use super::{err_from_u16, ErrorStatus};
use crate::process::stdio::{sysmeta_stderr, sysmeta_stdin, sysmeta_stdout};
use alloc::vec::Vec;
use core::ptr;
use safa_abi::raw::processes::{SpawnFlags, TaskStdio};
use safa_abi::raw::Optional;

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
    SyscallNum::SysOpen => {
        /// Opens the file with the path `path` and puts the resource id in `dest_fd`
        ///
        /// path must be valid utf-8
        sysopen(path_ptr: StrPtr, path_len: usize, dest_fd: *mut Ri)
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
    SyscallNum::SysClose => {
        /// Closes the file with the resource id `fd`
        sysclose(fd: Ri)
    }
}

#[inline]
/// Opens the path `path` and returns the resource id of the file descriptor
///
/// see [`sysopen`] for underlying syscall
pub fn open(path: &str) -> Result<Ri, ErrorStatus> {
    let mut dest_fd = 0xAAAAAAAAAAAAAAAAusize;
    err_from_u16!(
        sysopen(path.as_ptr(), path.len(), &raw mut dest_fd),
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
/// Closes the file with the resource id `fd`
///
/// see [`sysclose`] for underlying syscall
pub fn close(fd: Ri) -> Result<(), ErrorStatus> {
    err_from_u16!(sysclose(fd))
}

// Directory Iterator related syscalls
define_syscall! {
    SyscallNum::SysDirIterOpen =>
    {
        /// Opens a directory iterator for the directory with the resource id `dir_ri`
        sysdiriter_open(dir_ri: Ri, dest_ri: RequiredPtrMut<Ri>)
    },
    SyscallNum::SysDirIterClose => {
        /// Closes a directory iterator
        sysdiriter_close(fd: Ri)
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
/// Closes the directory iterator with the resource id `fd`
///
/// see [`sysdiriter_close`] for underlying syscall
pub fn diriter_close(fd: Ri) -> Result<(), ErrorStatus> {
    err_from_u16!(sysdiriter_close(fd))
}

#[inline]
/// Opens a directory iterator for the directory with the resource id `dir_ri`,
/// returns the resource id of the directory iterator
///
/// see [`sysdiriter_open`] for underlying syscall
pub fn diriter_open(dir_ri: Ri) -> Result<Ri, ErrorStatus> {
    let mut dest_fd: usize = 0xAAAAAAAAAAAAAAAAusize;
    err_from_u16!(sysdiriter_open(dir_ri, &raw mut dest_fd), dest_fd)
}

#[inline]
/// Gets the next directory entry from a directory iterator,
///
/// see [`sysdiriter_next`] for underlying syscall
pub fn diriter_next(dir_ri: Ri) -> Result<DirEntry, ErrorStatus> {
    let mut dest_direntry: DirEntry = unsafe { core::mem::zeroed() };
    err_from_u16!(
        sysdiriter_next(dir_ri, &raw mut dest_direntry),
        dest_direntry
    )
}

// File related syscalls
define_syscall! {
    SyscallNum::SysWrite => {
        /// Writes `len` bytes from `buf` to the file with the resource id `fd` at offset `offset`
        ///
        /// if `dest_wrote` is not null, it will be set to the number of bytes written
        syswrite(fd: Ri, offset: isize, buf: RequiredPtr<u8>, len: usize, dest_wrote: OptionalPtrMut<usize>)
    },
    SyscallNum::SysTruncate => {
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
    SyscallNum::SysRead => {
        /// Reads `len` bytes from the file with the resource id `fd` at offset `offset` into `buf`
        ///
        /// if `dest_read` is not null, it will be set to the number of bytes read
        sysread(fd: Ri, offset: isize, buf: RequiredPtrMut<u8>, len: usize, dest_read: OptionalPtrMut<usize>)
    },
    SyscallNum::SysSync => {
        /// Syncs the resource with the resource id `fd`
        syssync(ri: Ri)
    },
    SyscallNum::SysDup => {
        /// Duplicates the resource referred to by the resource id `ri` and puts the new resource id in `dest_ri`
        sysdup(ri: Ri, dest_ri: RequiredPtrMut<Ri>)
    }
}

#[inline]
/// Writes `buf.len()` bytes from `buf` to the file with the resource id `fd` at offset `offset`
/// and returns the number of bytes written
pub fn write(fd: Ri, offset: isize, buf: &[u8]) -> Result<usize, ErrorStatus> {
    let mut dest_wrote = 0;
    err_from_u16!(
        syswrite(fd, offset, buf.as_ptr(), buf.len(), &mut dest_wrote),
        dest_wrote
    )
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
    err_from_u16!(sysfsize(fd, &raw mut dest_size), dest_size)
}

#[inline]
/// Gets the file attributes of the file with the resource id `fd`
pub fn fattrs(fd: Ri) -> Result<FileAttr, ErrorStatus> {
    let mut attrs: FileAttr = unsafe { core::mem::zeroed() };
    err_from_u16!(sysfattrs(fd, &raw mut attrs), attrs)
}

#[inline]
/// Reads `buf.len()` bytes from the file with the resource id `fd` at offset `offset` into `buf`
pub fn read(fd: Ri, offset: isize, buf: &mut [u8]) -> Result<Ri, ErrorStatus> {
    let mut dest_read = 0;
    err_from_u16!(
        sysread(fd, offset, buf.as_mut_ptr(), buf.len(), &mut dest_read),
        dest_read
    )
}

#[inline]
/// Syncs the resource with the resource id `ri`
pub fn sync(ri: Ri) -> Result<(), ErrorStatus> {
    err_from_u16!(syssync(ri))
}

#[inline]
/// Duplicates the resource referred to by the resource id `ri`
/// and returns the new resource id
pub fn dup(ri: Ri) -> Result<Ri, ErrorStatus> {
    let mut dest_ri = 0xAAAAAAAAAAAAAAAAusize;
    err_from_u16!(sysdup(ri, &mut dest_ri), dest_ri)
}

// Process related syscalls
define_syscall! {
    SyscallNum::SysSbrk => {
        /// Increases the range of the process's data break by `size` bytes and puts the new break pointer in `target_ptr`
        syssbrk(size: isize, target_ptr: OptionalPtrMut<*mut u8>)
    },
    SyscallNum::SysExit => {
        /// Exits the process with the exit code `code`
        sysexit(code: usize) unreachable
    },
    SyscallNum::SysYield => {
        /// Switches to the next process/thread
        sysyield()
    },
    SyscallNum::SysWait => {
        /// Waits for the process with the resource id `pid` to exit
        /// if `exit_code` is not null, it will be set to the exit code of the process
        syswait(pid: Pid, exit_code: *mut usize)
    },
    SyscallNum::SysCHDir => {
        /// Changes the current working directory to the path `buf` with length `buf_len`
        /// (expects given buffer to be utf-8)
        syschdir(buf_ptr: StrPtr, buf_len: usize)
    },
    SyscallNum::SysGetCWD => {
        /// Gets the current working directory and puts it in `cwd_buf` with length `cwd_buf_len`
        /// if `dest_len` is not null, it will be set to the length of the cwd
        /// if the cwd is too long to fit in `cwd_buf`, the syscall will return [`ErrorStatus::Generic`] (1)
        /// the cwd is currently maximumally 1024 bytes
        sysgetcwd(cwd_buf: StrPtrMut, cwd_buf_len: usize, dest_len: OptionalPtrMut<usize>)
    }
}

#[inline]
/// Increases the range of the process's data break by `size` bytes
/// returns the new break pointer
///
/// you should probably use [`crate::alloc::GLOBAL_SYSTEM_ALLOCATOR`] instead for allocating memory
pub fn sbrk(size: isize) -> Result<*mut u8, ErrorStatus> {
    let mut target_ptr: *mut u8 = core::ptr::null_mut();
    err_from_u16!(syssbrk(size, &mut target_ptr), target_ptr)
}

#[inline]
pub fn exit(code: usize) -> ! {
    sysexit(code)
}

#[inline]
pub fn yield_now() {
    debug_assert!(sysyield().is_success())
}

#[inline]
/// Waits for the process with the resource id `pid` to exit
/// and returns the exit code of the process
///
/// will return 0 as an exit code if process wasn't found
pub fn wait(pid: Pid) -> Result<usize, ErrorStatus> {
    let mut dest_exit_code = 0;
    err_from_u16!(syswait(pid, &mut dest_exit_code), dest_exit_code)
}

#[inline]
/// Changes the current work dir to `path`
pub fn chdir(path: &str) -> Result<(), ErrorStatus> {
    let path = path.as_bytes();
    err_from_u16!(syschdir(path.as_ptr(), path.len()))
}

use alloc::string::String;
#[inline]
/// Retrieves the current work dir
pub fn getcwd() -> Result<String, ErrorStatus> {
    let mut buffer = Vec::with_capacity(safa_abi::consts::MAX_PATH_LENGTH);
    unsafe {
        buffer.set_len(buffer.capacity());
    }

    let mut dest_len: usize = 0xAAAAAAAAAAAAAAAAusize;
    let len = err_from_u16!(
        sysgetcwd(buffer.as_mut_ptr(), buffer.len(), &raw mut dest_len),
        dest_len
    )?;

    buffer.truncate(len);
    unsafe { Ok(String::from_utf8_unchecked(buffer)) }
}

define_syscall! {
    SyscallNum::SysShutdown => {
        /// Shuts down the system
        sysshutdown() unreachable
    },
    SyscallNum::SysReboot => {
        /// Reboots the system
        sysreboot() unreachable
    }
}

#[inline]
pub fn shutdown() -> ! {
    sysshutdown()
}

#[inline]
pub fn reboot() -> ! {
    sysreboot()
}

// doesn't use define_syscall because we use a different signature then the rest of the syscalls
#[cfg_attr(
    not(any(feature = "std", feature = "rustc-dep-of-std")),
    unsafe(no_mangle)
)]
#[inline(always)]
/// Spawns a new process with the path `path` with arguments `argv` and flags `flags`
/// name_ptr can be null, in which case the name will be the path
/// path and name must be valid utf-8
/// if `dest_pid` is not null, it will be set to the pid of the new process
/// if `stdin`, `stdout`, or `stderr` are not `None`, the corresponding file descriptors will be inherited from the parent
/// if they are None they will be inherited from the parent
extern "C" fn syspspawn(
    name_ptr: OptionalStrPtr,
    name_len: usize,
    path_ptr: StrPtr,
    path_len: usize,
    argv_ptr: OptionalPtrMut<RawSlice<u8>>,
    argv_len: usize,
    flags: SpawnFlags,
    dest_pid: OptionalPtrMut<Pid>,
    stdin: Optional<Ri>,
    stdout: Optional<Ri>,
    stderr: Optional<Ri>,
) -> SyscallResult {
    use safa_abi::raw::processes::SpawnConfig;
    let (mut stdin, mut stdout, mut stderr): (Option<_>, Option<_>, Option<_>) =
        (stdin.into(), stdout.into(), stderr.into());

    let stdio = {
        if stdin.is_none() && stdout.is_none() && stderr.is_none() {
            None
        } else {
            stdout.get_or_insert_with(|| sysmeta_stdout());
            stdin.get_or_insert_with(|| sysmeta_stdin());
            stderr.get_or_insert_with(|| sysmeta_stderr());

            Some(TaskStdio::new(stdout, stdin, stderr))
        }
    };

    let stdio = stdio.as_ref();
    let stdio_ptr = stdio.map(|m| m as *const _).unwrap_or(core::ptr::null());
    let (_, mut env) = unsafe { crate::process::env::duplicate_env() };

    let config = SpawnConfig {
        version: 1,
        name: unsafe { RawSlice::from_raw_parts(name_ptr, name_len) },
        argv: unsafe { RawSliceMut::from_raw_parts(argv_ptr, argv_len) },
        env: unsafe { RawSliceMut::from_raw_parts(env.as_mut_ptr(), env.len()) },
        flags,
        stdio: stdio_ptr,
    };

    syscall!(
        SyscallNum::SysPSpawn,
        path_ptr as usize,
        path_len,
        (&raw const config) as usize,
        dest_pid as *mut _ as usize,
    )
}

/// spawns a new process
/// # Arguments
/// * `stdin`, `stdout`, `stderr` are the file descriptors of stdio, if None, they will be inherited from the parent
/// # Safety
/// `argv` must be valid pointers to a slice of slices of `&str`
/// `argv` will become invalid after use, using them is UB
#[inline]
pub unsafe fn unsafe_pspawn(
    name: Option<&str>,
    path: &str,
    argv: *mut [&str],
    flags: SpawnFlags,
    stdin: Option<Ri>,
    stdout: Option<Ri>,
    stderr: Option<Ri>,
) -> Result<Pid, ErrorStatus> {
    let mut pid = 0;

    let name = name.map(|s| s.as_bytes());
    let name_ptr = name.map(|s| s.as_ptr()).unwrap_or(ptr::null());
    let name_len = name.map(|s| s.len()).unwrap_or(0);

    let argv = argv as *mut [&[u8]];
    let argv = unsafe { RawSliceMut::from_slices(argv) };

    err_from_u16!(
        syspspawn(
            name_ptr,
            name_len,
            path.as_ptr(),
            path.len(),
            argv.as_mut_ptr(),
            argv.len(),
            flags,
            &mut pid,
            stdin.into(),
            stdout.into(),
            stderr.into(),
        ),
        pid
    )
}

// FIXME: I convert the argv form &[u8] to RawSlice
// and that is why it is consumed,
// i reuse the argv buffer as the result of the conversion
// even though this might be inefficient especially that RawSlice should have the same layout as &[u8] and same for env
// although this is fine because this method is only used in the rust standard library which gives args as an owned Vec anyways
//
/// same as [`unsafe_pspawn`] but safe because it makes it clear that `argv`  are consumed
#[inline]
pub fn pspawn(
    name: Option<&str>,
    path: &str,
    mut argv: Vec<&str>,
    flags: SpawnFlags,
    stdin: Option<Ri>,
    stdout: Option<Ri>,
    stderr: Option<Ri>,
) -> Result<Pid, ErrorStatus> {
    let argv: &mut [&str] = &mut argv;
    unsafe { unsafe_pspawn(name, path, argv as *mut _, flags, stdin, stdout, stderr) }
}

define_syscall! {
    SyscallNum::SysUptime => {
        /// returns the system uptime in milliseconds
        sysuptime(uptime: RequiredPtrMut<u64>)
    }
}

#[inline]
pub fn uptime() -> u64 {
    let mut results: u64 = 0;
    sysuptime(&mut results);
    results
}
