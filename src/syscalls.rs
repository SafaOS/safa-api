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

#[cfg_attr(
    not(any(feature = "std", feature = "rustc-dep-of-std")),
    unsafe(no_mangle)
)]
#[inline(always)]
extern "C" fn sysgetdirentry(
    path_ptr: *const u8,
    path_len: usize,
    dest_direntry: *mut DirEntry,
) -> u16 {
    syscall3(
        SyscallNum::SysGetDirEntry,
        path_ptr as usize,
        path_len,
        dest_direntry as usize,
    )
}

#[inline]
pub fn getdirentry(path: &str) -> Result<DirEntry, ErrorStatus> {
    let mut dest_direntry: DirEntry = unsafe { core::mem::zeroed() };
    err_from_u16!(
        sysgetdirentry(path.as_ptr(), path.len(), &raw mut dest_direntry),
        dest_direntry
    )
}

#[cfg_attr(
    not(any(feature = "std", feature = "rustc-dep-of-std")),
    unsafe(no_mangle)
)]
#[inline(always)]
extern "C" fn sysopen(path_ptr: *const u8, path_len: usize, dest_fd: *mut usize) -> u16 {
    syscall3(
        SyscallNum::SysOpen,
        path_ptr as usize,
        path_len,
        dest_fd as usize,
    )
}

#[cfg_attr(
    not(any(feature = "std", feature = "rustc-dep-of-std")),
    unsafe(no_mangle)
)]
#[inline(always)]
extern "C" fn syscreate_file(path_ptr: *const u8, path_len: usize) -> u16 {
    syscall2(SyscallNum::SysCreate, path_ptr as usize, path_len)
}

#[inline]
pub fn create(path: &str) -> Result<(), ErrorStatus> {
    err_from_u16!(syscreate_file(path.as_ptr(), path.len()))
}

#[cfg_attr(
    not(any(feature = "std", feature = "rustc-dep-of-std")),
    unsafe(no_mangle)
)]
#[inline(always)]
extern "C" fn syscreate_dir(path_ptr: *const u8, path_len: usize) -> u16 {
    syscall2(SyscallNum::SysCreateDir, path_ptr as usize, path_len)
}

#[inline]
pub fn createdir(path: &str) -> Result<(), ErrorStatus> {
    err_from_u16!(syscreate_dir(path.as_ptr(), path.len()))
}

#[inline]
pub fn open(path: &str) -> Result<usize, ErrorStatus> {
    let mut dest_fd = 0xAAAAAAAAAAAAAAAAusize;
    err_from_u16!(
        sysopen(path.as_ptr(), path.len(), &raw mut dest_fd),
        dest_fd
    )
}

#[cfg_attr(
    not(any(feature = "std", feature = "rustc-dep-of-std")),
    unsafe(no_mangle)
)]
#[inline(always)]
extern "C" fn sysclose(fd: usize) -> u16 {
    syscall1(SyscallNum::SysClose, fd)
}

#[inline]
pub fn close(fd: usize) -> Result<(), ErrorStatus> {
    err_from_u16!(sysclose(fd))
}

#[cfg_attr(
    not(any(feature = "std", feature = "rustc-dep-of-std")),
    unsafe(no_mangle)
)]
#[inline(always)]
extern "C" fn sysdiriter_close(fd: usize) -> u16 {
    syscall1(SyscallNum::SysDirIterClose, fd)
}

#[inline]
pub fn diriter_close(fd: usize) -> Result<(), ErrorStatus> {
    err_from_u16!(sysdiriter_close(fd))
}

#[cfg_attr(
    not(any(feature = "std", feature = "rustc-dep-of-std")),
    unsafe(no_mangle)
)]
#[inline(always)]
extern "C" fn sysdiriter_open(dir_ri: usize, dest_ri: *mut usize) -> u16 {
    syscall2(SyscallNum::SysDirIterOpen, dir_ri, dest_ri as usize)
}

#[inline]
pub fn diriter_open(dir_ri: usize) -> Result<usize, ErrorStatus> {
    let mut dest_fd: usize = 0xAAAAAAAAAAAAAAAAusize;
    err_from_u16!(sysdiriter_open(dir_ri, &raw mut dest_fd), dest_fd)
}

#[cfg_attr(
    not(any(feature = "std", feature = "rustc-dep-of-std")),
    unsafe(no_mangle)
)]
#[inline(always)]
extern "C" fn sysdiriter_next(dir_ri: usize, dest_direntry: *mut DirEntry) -> u16 {
    syscall2(SyscallNum::SysDirIterNext, dir_ri, dest_direntry as usize)
}

#[inline]
pub fn diriter_next(dir_ri: usize) -> Result<DirEntry, ErrorStatus> {
    let mut dest_direntry: DirEntry = unsafe { core::mem::zeroed() };
    err_from_u16!(
        sysdiriter_next(dir_ri, &raw mut dest_direntry),
        dest_direntry
    )
}

#[cfg_attr(
    not(any(feature = "std", feature = "rustc-dep-of-std")),
    unsafe(no_mangle)
)]
#[inline(always)]
extern "C" fn syswrite(
    fd: usize,
    offset: isize,
    buf: *const u8,
    len: usize,
    dest_wrote: &mut usize,
) -> u16 {
    syscall5(
        SyscallNum::SysWrite,
        fd,
        offset as usize,
        buf as usize,
        len,
        dest_wrote as *mut _ as usize,
    )
}

#[inline]
pub fn write(fd: usize, offset: isize, buf: &[u8]) -> Result<usize, ErrorStatus> {
    let mut dest_wrote = 0;
    err_from_u16!(
        syswrite(fd, offset, buf.as_ptr(), buf.len(), &mut dest_wrote),
        dest_wrote
    )
}

#[cfg_attr(
    not(any(feature = "std", feature = "rustc-dep-of-std")),
    unsafe(no_mangle)
)]
#[inline(always)]
extern "C" fn systruncate(fd: usize, len: usize) -> u16 {
    syscall2(SyscallNum::SysTruncate, fd, len)
}

#[inline]
pub fn truncate(fd: usize, len: usize) -> Result<(), ErrorStatus> {
    err_from_u16!(systruncate(fd, len))
}

#[cfg_attr(
    not(any(feature = "std", feature = "rustc-dep-of-std")),
    unsafe(no_mangle)
)]
#[inline(always)]
pub fn sysfsize(fd: usize, dest_size: *mut usize) -> u16 {
    syscall2(SyscallNum::SysFSize, fd, dest_size as usize)
}

#[inline]
pub fn fsize(fd: usize) -> Result<usize, ErrorStatus> {
    let mut dest_size = 0;
    err_from_u16!(sysfsize(fd, &raw mut dest_size), dest_size)
}

#[cfg_attr(
    not(any(feature = "std", feature = "rustc-dep-of-std")),
    unsafe(no_mangle)
)]
#[inline(always)]
extern "C" fn sysfattrs(fd: usize, dest_attrs: *mut FileAttr) -> u16 {
    syscall2(SyscallNum::SysFAttrs, fd, dest_attrs as usize)
}

#[inline]
pub fn fattrs(fd: usize) -> Result<FileAttr, ErrorStatus> {
    let mut attrs: FileAttr = unsafe { core::mem::zeroed() };
    err_from_u16!(sysfattrs(fd, &raw mut attrs), attrs)
}

#[cfg_attr(
    not(any(feature = "std", feature = "rustc-dep-of-std")),
    unsafe(no_mangle)
)]
#[inline(always)]
extern "C" fn sysread(
    fd: usize,
    offset: isize,
    buf: *mut u8,
    len: usize,
    dest_read: &mut usize,
) -> u16 {
    syscall5(
        SyscallNum::SysRead,
        fd,
        offset as usize,
        buf as usize,
        len,
        dest_read as *mut _ as usize,
    )
}

#[inline]
pub fn read(fd: usize, offset: isize, buf: &mut [u8]) -> Result<usize, ErrorStatus> {
    let mut dest_read = 0;
    err_from_u16!(
        sysread(fd, offset, buf.as_mut_ptr(), buf.len(), &mut dest_read),
        dest_read
    )
}

#[cfg_attr(
    not(any(feature = "std", feature = "rustc-dep-of-std")),
    unsafe(no_mangle)
)]
#[inline(always)]
extern "C" fn syssync(fd: usize) -> u16 {
    syscall1(SyscallNum::SysSync, fd)
}

#[inline]
pub fn sync(fd: usize) -> Result<(), ErrorStatus> {
    err_from_u16!(syssync(fd))
}
#[cfg_attr(
    not(any(feature = "std", feature = "rustc-dep-of-std")),
    unsafe(no_mangle)
)]
#[inline(always)]
extern "C" fn syssbrk(size: isize, target_ptr: &mut *mut u8) -> u16 {
    syscall2(
        SyscallNum::SysSbrk,
        size as usize,
        target_ptr as *mut _ as usize,
    )
}

#[inline]
pub fn sbrk(size: isize) -> Result<*mut u8, ErrorStatus> {
    let mut target_ptr: *mut u8 = core::ptr::null_mut();
    err_from_u16!(syssbrk(size, &mut target_ptr), target_ptr)
}

#[cfg_attr(
    not(any(feature = "std", feature = "rustc-dep-of-std")),
    unsafe(no_mangle)
)]
#[inline(always)]
extern "C" fn sysexit(code: usize) -> ! {
    syscall1(SyscallNum::SysExit, code);
    unreachable!()
}

#[cfg_attr(
    not(any(feature = "std", feature = "rustc-dep-of-std")),
    unsafe(no_mangle)
)]
#[inline(always)]
extern "C" fn sysyield() -> u16 {
    syscall0(SyscallNum::SysYield)
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
    sysexit(code)
}
#[cfg_attr(
    not(any(feature = "std", feature = "rustc-dep-of-std")),
    unsafe(no_mangle)
)]
#[inline(always)]
extern "C" fn syschdir(buf_ptr: *const u8, buf_len: usize) -> u16 {
    syscall2(SyscallNum::SysCHDir, buf_ptr as usize, buf_len)
}

#[inline]
pub fn chdir(path: &str) -> Result<(), ErrorStatus> {
    let path = path.as_bytes();
    err_from_u16!(syschdir(path.as_ptr(), path.len()))
}

#[cfg_attr(
    not(any(feature = "std", feature = "rustc-dep-of-std")),
    unsafe(no_mangle)
)]
/// Gets the current working directory
/// returns Err(ErrorStatus::Generic) if the buffer is too small to hold the cwd
#[inline(always)]
extern "C" fn sysgetcwd(cwd_buf_ptr: *mut u8, cwd_buf_len: usize, dest_len: &mut usize) -> u16 {
    syscall3(
        SyscallNum::SysGetCWD,
        cwd_buf_ptr as usize,
        cwd_buf_len,
        dest_len as *mut _ as usize,
    )
}

#[inline]
pub fn getcwd() -> Result<Vec<u8>, ErrorStatus> {
    let do_syscall = |cwd_buf: &mut [u8]| {
        let mut dest_len = 0;
        err_from_u16!(
            sysgetcwd(cwd_buf.as_mut_ptr(), cwd_buf.len(), &mut dest_len),
            dest_len
        )
    };

    let extend = |cwd_buf: &mut Vec<u8>| unsafe {
        cwd_buf.reserve(128);
        cwd_buf.set_len(cwd_buf.capacity());
    };

    let mut cwd_buf = Vec::new();
    extend(&mut cwd_buf);

    loop {
        match do_syscall(&mut cwd_buf) {
            Ok(len) => unsafe {
                cwd_buf.set_len(len);
                return Ok(cwd_buf);
            },
            Err(err) => {
                if err == ErrorStatus::Generic {
                    extend(&mut cwd_buf);
                } else {
                    return Err(err);
                }
            }
        }
    }
}

#[cfg_attr(
    not(any(feature = "std", feature = "rustc-dep-of-std")),
    unsafe(no_mangle)
)]
#[inline(always)]
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
