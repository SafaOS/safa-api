use crate::process::{sysmeta_stderr, sysmeta_stdin, sysmeta_stdout};
use crate::raw::io::DirEntry;

use super::errors::ErrorStatus;

#[cfg(not(feature = "rustc-dep-of-std"))]
extern crate alloc;
use super::raw::io::FileAttr;
use super::raw::{RawSlice, RawSliceMut};
use alloc::vec::Vec;
use core::arch::asm;
use core::ptr;
use safa_abi::raw::processes::SpawnFlags;
use safa_abi::raw::Optional;
pub use safa_abi::syscalls::SyscallTable as SyscallNum;

macro_rules! err_from_u16 {
    ($result:expr) => {
        unsafe {
            Into::<Result<(), $crate::errors::ErrorStatus>>::into(
                TryInto::<$crate::errors::SysResult>::try_into($result).unwrap_unchecked(),
            )
        }
    };
    ($result:expr, $ok:expr) => {
        err_from_u16!($result).map(|()| $ok)
    };
}

pub(crate) use err_from_u16;

#[inline(always)]
pub fn syscall0(num: SyscallNum) -> u16 {
    let result: u16;
    unsafe {
        asm!(
            "int 0x80",
            in("rax") num as usize,
            lateout("rax") result,
        );
        result
    }
}

#[inline(always)]
pub fn syscall1(num: SyscallNum, arg1: usize) -> u16 {
    let result: u16;
    unsafe {
        asm!(
            "int 0x80",
            in("rax") num as usize,
            in("rdi") arg1,
            lateout("rax") result,
        );
        result
    }
}

#[inline(always)]
pub fn syscall2(num: SyscallNum, arg1: usize, arg2: usize) -> u16 {
    let result: u16;
    unsafe {
        asm!(
            "int 0x80",
            in("rax") num as usize,
            in("rdi") arg1,
            in("rsi") arg2,
            lateout("rax") result,
        );
        result
    }
}

#[inline(always)]
pub fn syscall3(num: SyscallNum, arg1: usize, arg2: usize, arg3: usize) -> u16 {
    let result: u16;
    unsafe {
        asm!(
            "int 0x80",
            in("rax") num as usize,
            in("rdi") arg1,
            in("rsi") arg2,
            in("rdx") arg3,
            lateout("rax") result,
        );
        result
    }
}

#[inline(always)]
pub fn syscall5(
    num: SyscallNum,
    arg1: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
    arg5: usize,
) -> u16 {
    let result: u16;
    unsafe {
        asm!(
            "int 0x80",
            in("rax") num as usize,
            in("rdi") arg1,
            in("rsi") arg2,
            in("rdx") arg3,
            in("rcx") arg4,
            in("r8") arg5,
            lateout("rax") result,
        );
        result
    }
}

#[inline(always)]
pub fn syscall4(num: SyscallNum, arg1: usize, arg2: usize, arg3: usize, arg4: usize) -> u16 {
    let result: u16;
    unsafe {
        asm!(
            "int 0x80",
            in("rax") num as usize,
            in("rdi") arg1,
            in("rsi") arg2,
            in("rdx") arg3,
            in("rcx") arg4,
            lateout("rax") result,
        );
        result
    }
}

macro_rules! syscall {
    ($num: path $(,)?) => {
        $crate::syscalls::syscall0($num)
    };
    ($num: path, $arg1: expr) => {
        $crate::syscalls::syscall1($num, $arg1)
    };
    ($num: path, $arg1: expr, $arg2: expr) => {
        $crate::syscalls::syscall2($num, $arg1, $arg2)
    };
    ($num: path, $arg1: expr, $arg2: expr, $arg3: expr) => {
        $crate::syscalls::syscall3($num, $arg1, $arg2, $arg3)
    };
    ($num: path, $arg1: expr, $arg2: expr, $arg3: expr, $arg4: expr) => {
        $crate::syscalls::syscall4($num, $arg1, $arg2, $arg3, $arg4)
    };
    ($num: path, $arg1: expr, $arg2: expr, $arg3: expr, $arg4: expr, $arg5: expr) => {
        $crate::syscalls::syscall5($num, $arg1, $arg2, $arg3, $arg4, $arg5)
    };
}

pub(crate) use syscall;

macro_rules! define_syscall {
    ($num:path => { $(#[$($attrss:tt)*])* $name:ident ($($arg:ident : $ty:ty),*) }) => {
        #[cfg_attr(
            not(any(feature = "std", feature = "rustc-dep-of-std")),
            unsafe(no_mangle)
        )]
        #[inline(always)]
        $(#[$($attrss)*])*
        extern "C" fn $name($($arg: $ty),*) -> u16 {
            let result = $crate::syscalls::syscall!($num, $( $arg as usize),*);
            result
        }
    };
    {$($num:path => { $(#[$($attrss:tt)*])* $name:ident ($($arg:ident: $ty:ty),*) }),*} => {
        $(define_syscall!($num => { $name($($arg: $ty),*) });)*
    };
}

pub(crate) use define_syscall;

define_syscall!(SyscallNum::SysGetDirEntry => {
    /// Gets the directory entry for the path `path` and puts it in `dest_direntry`
    /// path must be valid utf-8
    /// `dest_direntry` can be null
    sysgetdirentry(path_ptr: *const u8, path_len: usize, dest_direntry: *mut DirEntry)
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
        /// path must be valid utf-8
        sysopen(path_ptr: *const u8, path_len: usize, dest_fd: *mut usize)
    },
    SyscallNum::SysCreate => {
        /// Creates the file with the path `path`
        /// path must be valid utf-8
        syscreate_file(path_ptr: *const u8, path_len: usize)
    },
    SyscallNum::SysCreateDir => {
        /// Creates the directory with the path `path`
        /// path must be valid utf-8
        syscreate_dir(path_ptr: *const u8, path_len: usize)
    },
    SyscallNum::SysClose => {
        /// Closes the file with the resource id `fd`
        sysclose(fd: usize)
    }
}

#[inline]
pub fn open(path: &str) -> Result<usize, ErrorStatus> {
    let mut dest_fd = 0xAAAAAAAAAAAAAAAAusize;
    err_from_u16!(
        sysopen(path.as_ptr(), path.len(), &raw mut dest_fd),
        dest_fd
    )
}

#[inline]
pub fn create(path: &str) -> Result<(), ErrorStatus> {
    err_from_u16!(syscreate_file(path.as_ptr(), path.len()))
}

#[inline]
pub fn createdir(path: &str) -> Result<(), ErrorStatus> {
    err_from_u16!(syscreate_dir(path.as_ptr(), path.len()))
}

#[inline]
pub fn close(fd: usize) -> Result<(), ErrorStatus> {
    err_from_u16!(sysclose(fd))
}

// Directory Iterator related syscalls
define_syscall! {
    SyscallNum::SysDirIterOpen =>
    {
        /// Opens a directory iterator for the directory with the resource id `dir_ri`
        sysdiriter_open(dir_ri: usize, dest_ri: *mut usize)
    },
    SyscallNum::SysDirIterClose => {
        /// Closes a directory iterator
        sysdiriter_close(fd: usize)
    },
    SyscallNum::SysDirIterNext => {
        /// Gets the next directory entry from a directory iterator,
        /// puts the results in `dest_direntry`,
        /// puts zeroed DirEntry in `dest_direntry` if there are no more entries
        sysdiriter_next(dir_ri: usize, dest_direntry: *mut DirEntry)
    }
}

#[inline]
pub fn diriter_close(fd: usize) -> Result<(), ErrorStatus> {
    err_from_u16!(sysdiriter_close(fd))
}

#[inline]
pub fn diriter_open(dir_ri: usize) -> Result<usize, ErrorStatus> {
    let mut dest_fd: usize = 0xAAAAAAAAAAAAAAAAusize;
    err_from_u16!(sysdiriter_open(dir_ri, &raw mut dest_fd), dest_fd)
}

#[inline]
pub fn diriter_next(dir_ri: usize) -> Result<DirEntry, ErrorStatus> {
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
        syswrite(fd: usize, offset: isize, buf: *const u8, len: usize, dest_wrote: *mut usize)
    },
    SyscallNum::SysTruncate => {
        /// Truncates the file with the resource id `fd` to `len` bytes
        systruncate(fd: usize, len: usize)
    },
    SyscallNum::SysFSize => {
        /// Gets the size of the file with the resource id `fd` and puts it in `dest_size`
        sysfsize(fd: usize, dest_size: *mut usize)
    },
    SyscallNum::SysFAttrs => {
        /// Gets the file attributes of the file with the resource id `fd` and puts them in `dest_attrs`
        sysfattrs(fd: usize, dest_attrs: *mut FileAttr)
    },
    SyscallNum::SysRead => {
        /// Reads `len` bytes from the file with the resource id `fd` at offset `offset` into `buf`
        sysread(fd: usize, offset: isize, buf: *mut u8, len: usize, dest_read: *mut usize)
    },
    SyscallNum::SysSync => {
        /// Syncs the file with the resource id `fd`
        syssync(fd: usize)
    },
    SyscallNum::SysDup => {
        /// Duplicates the file with the resource id `fd` and puts the new resource id in `dest_fd`
        sysdup(fd: usize, dest_fd: *mut usize)
    }
}

#[inline]
pub fn write(fd: usize, offset: isize, buf: &[u8]) -> Result<usize, ErrorStatus> {
    let mut dest_wrote = 0;
    err_from_u16!(
        syswrite(fd, offset, buf.as_ptr(), buf.len(), &mut dest_wrote),
        dest_wrote
    )
}

#[inline]
pub fn truncate(fd: usize, len: usize) -> Result<(), ErrorStatus> {
    err_from_u16!(systruncate(fd, len))
}

#[inline]
pub fn fsize(fd: usize) -> Result<usize, ErrorStatus> {
    let mut dest_size = 0;
    err_from_u16!(sysfsize(fd, &raw mut dest_size), dest_size)
}

#[inline]
pub fn fattrs(fd: usize) -> Result<FileAttr, ErrorStatus> {
    let mut attrs: FileAttr = unsafe { core::mem::zeroed() };
    err_from_u16!(sysfattrs(fd, &raw mut attrs), attrs)
}

#[inline]
pub fn read(fd: usize, offset: isize, buf: &mut [u8]) -> Result<usize, ErrorStatus> {
    let mut dest_read = 0;
    err_from_u16!(
        sysread(fd, offset, buf.as_mut_ptr(), buf.len(), &mut dest_read),
        dest_read
    )
}

#[inline]
pub fn sync(fd: usize) -> Result<(), ErrorStatus> {
    err_from_u16!(syssync(fd))
}

#[inline]
pub fn dup(fd: usize) -> Result<usize, ErrorStatus> {
    let mut dest_fd = 0xAAAAAAAAAAAAAAAAusize;
    err_from_u16!(sysdup(fd, &mut dest_fd), dest_fd)
}

// Process related syscalls
define_syscall! {
    SyscallNum::SysSbrk => {
        /// Increases the range of the process's data break by `size` bytes and puts the new break pointer in `target_ptr`
        syssbrk(size: isize, target_ptr: *mut *mut u8)
    },
    SyscallNum::SysExit => {
        /// Exits the process with the exit code `code`
        sysexit(code: usize)
    },
    SyscallNum::SysYield => {
        /// Switches to the next process/thread
        sysyield()
    },
    SyscallNum::SysCHDir => {
        /// Changes the current working directory to the path `buf` with length `buf_len`
        /// (expects given buffer to be utf-8)
        syschdir(buf_ptr: *const u8, buf_len: usize)
    },
    SyscallNum::SysGetCWD => {
        /// Gets the current working directory and puts it in `cwd_buf` with length `cwd_buf_len`
        /// if `dest_len` is not null, it will be set to the length of the cwd
        /// if the cwd is too long to fit in `cwd_buf`, the syscall will return [`ErrorStatus::Generic`] (1)
        /// the cwd is currently maximumally 1024 bytes
        sysgetcwd(cwd_buf: *mut u8, cwd_buf_len: usize, dest_len: *mut usize)
    }
}

#[inline]
pub fn sbrk(size: isize) -> Result<*mut u8, ErrorStatus> {
    let mut target_ptr: *mut u8 = core::ptr::null_mut();
    err_from_u16!(syssbrk(size, &mut target_ptr), target_ptr)
}

#[inline]
pub fn yield_now() {
    debug_assert!(sysyield() == 0)
}

#[cfg_attr(
    not(any(feature = "std", feature = "rustc-dep-of-std")),
    unsafe(no_mangle)
)]
#[inline(always)]
extern "C" fn sysshutdown() -> ! {
    syscall0(SyscallNum::SysShutdown);
    unreachable!()
}

#[inline]
pub fn shutdown() -> ! {
    sysshutdown()
}

#[cfg_attr(
    not(any(feature = "std", feature = "rustc-dep-of-std")),
    unsafe(no_mangle)
)]
#[inline(always)]
extern "C" fn sysreboot() -> ! {
    syscall0(SyscallNum::SysReboot);
    unreachable!()
}

#[inline]
pub fn reboot() -> ! {
    sysreboot()
}

#[inline]
pub fn exit(code: usize) -> ! {
    sysexit(code);
    unreachable!()
}

#[inline]
pub fn chdir(path: &str) -> Result<(), ErrorStatus> {
    let path = path.as_bytes();
    err_from_u16!(syschdir(path.as_ptr(), path.len()))
}

#[inline]
pub fn getcwd() -> Result<Vec<u8>, ErrorStatus> {
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
    Ok(buffer)
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
    name_ptr: *const u8,
    name_len: usize,
    path_ptr: *const u8,
    path_len: usize,
    argv_ptr: *mut RawSlice<u8>,
    argv_len: usize,
    flags: SpawnFlags,
    dest_pid: &mut usize,
    stdin: Optional<usize>,
    stdout: Optional<usize>,
    stderr: Optional<usize>,
) -> u16 {
    use safa_abi::raw::processes::SpawnConfig;
    use safa_abi::raw::processes::TaskMetadata;
    let (mut stdin, mut stdout, mut stderr): (Option<_>, Option<_>, Option<_>) =
        (stdin.into(), stdout.into(), stderr.into());

    let metadata = {
        if stdin.is_none() && stdout.is_none() && stderr.is_none() {
            None
        } else {
            stdout.get_or_insert_with(|| sysmeta_stdout());
            stdin.get_or_insert_with(|| sysmeta_stdin());
            stderr.get_or_insert_with(|| sysmeta_stderr());

            Some(TaskMetadata::new(stdout, stdin, stderr))
        }
    };

    let metadata = metadata.as_ref();
    let meta_ptr = metadata.map(|m| m as *const _).unwrap_or(core::ptr::null());

    let config = SpawnConfig {
        version: 1,
        name: unsafe { RawSlice::from_raw_parts(name_ptr, name_len) },
        argv: unsafe { RawSliceMut::from_raw_parts(argv_ptr, argv_len) },
        flags,
        metadata: meta_ptr,
    };
    syscall4(
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
/// `argv` must be a valid pointer to a slice of slices of `&str`
/// `argv` will become invaild after use, using it is UB
#[inline]
pub unsafe fn unsafe_pspawn(
    name: Option<&str>,
    path: &str,
    argv: *mut [&str],
    flags: SpawnFlags,
    stdin: Option<usize>,
    stdout: Option<usize>,
    stderr: Option<usize>,
) -> Result<usize, ErrorStatus> {
    let mut pid = 0;

    let name = name.map(|s| s.as_bytes());
    let name_ptr = name.map(|s| s.as_ptr()).unwrap_or(ptr::null());
    let name_len = name.map(|s| s.len()).unwrap_or(0);

    let argv: *mut [&[u8]] = argv as *mut [&[u8]];
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

/// same as [`unsafe_pspawn`] but safe because it makes it clear that `argv` is consumed`
#[inline]
pub fn pspawn(
    name: Option<&str>,
    path: &str,
    mut argv: Vec<&str>,
    flags: SpawnFlags,
    stdin: Option<usize>,
    stdout: Option<usize>,
    stderr: Option<usize>,
) -> Result<usize, ErrorStatus> {
    let argv: &mut [&str] = &mut argv;
    unsafe { unsafe_pspawn(name, path, argv as *mut _, flags, stdin, stdout, stderr) }
}
#[cfg_attr(
    not(any(feature = "std", feature = "rustc-dep-of-std")),
    unsafe(no_mangle)
)]
#[inline(always)]
extern "C" fn syswait(pid: usize, exit_code: &mut usize) -> u16 {
    syscall2(SyscallNum::SysWait, pid, exit_code as *mut _ as usize)
}

#[inline]
pub fn wait(pid: usize) -> Result<usize, ErrorStatus> {
    let mut dest_exit_code = 0;
    err_from_u16!(syswait(pid, &mut dest_exit_code), dest_exit_code)
}
