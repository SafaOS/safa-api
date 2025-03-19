use super::errors::ErrorStatus;

#[cfg(not(feature = "rustc-dep-of-std"))]
extern crate alloc;
use super::raw::{RawSlice, RawSliceMut};
use alloc::vec::Vec;
use core::arch::asm;
use core::{ops, ptr};
use safa_utils::errors::SysResult;
pub use safa_utils::syscalls::SyscallTable as SyscallNum;

macro_rules! err_from_u16 {
    ($result:expr) => {
        unsafe {
            Into::<Result<(), ErrorStatus>>::into(
                TryInto::<SysResult>::try_into($result).unwrap_unchecked(),
            )
        }
    };
    ($result:expr, $ok:expr) => {
        err_from_u16!($result).map(|()| $ok)
    };
}

#[inline(always)]
fn syscall1(num: SyscallNum, arg1: usize) -> u16 {
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
fn syscall2(num: SyscallNum, arg1: usize, arg2: usize) -> u16 {
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
fn syscall3(num: SyscallNum, arg1: usize, arg2: usize, arg3: usize) -> u16 {
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
fn syscall5(
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
fn syscall4(num: SyscallNum, arg1: usize, arg2: usize, arg3: usize, arg4: usize) -> u16 {
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

#[inline(always)]
#[unsafe(no_mangle)]
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

#[unsafe(no_mangle)]
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

#[unsafe(no_mangle)]
#[inline(always)]
extern "C" fn syssync(fd: usize) -> u16 {
    syscall1(SyscallNum::SysSync, fd)
}

#[inline]
pub fn sync(fd: usize) -> Result<(), ErrorStatus> {
    err_from_u16!(syssync(fd))
}

#[unsafe(no_mangle)]
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

#[unsafe(no_mangle)]
#[inline(always)]
extern "C" fn sysexit(code: usize) -> ! {
    syscall1(SyscallNum::SysExit, code);
    unreachable!()
}

#[inline]
pub fn exit(code: usize) -> ! {
    sysexit(code)
}

#[unsafe(no_mangle)]
#[inline(always)]
extern "C" fn syschdir(buf_ptr: *const u8, buf_len: usize) -> u16 {
    syscall2(SyscallNum::SysCHDir, buf_ptr as usize, buf_len)
}

#[inline]
pub fn chdir(path: &str) -> Result<(), ErrorStatus> {
    let path = path.as_bytes();
    err_from_u16!(syschdir(path.as_ptr(), path.len()))
}

/// Gets the current working directory
/// returns Err(ErrorStatus::Generic) if the buffer is too small to hold the cwd
#[unsafe(no_mangle)]
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

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SpawnFlags(u8);
impl SpawnFlags {
    pub const CLONE_RESOURCES: Self = Self(1 << 0);
    pub const CLONE_CWD: Self = Self(1 << 1);
}

impl ops::BitOr for SpawnFlags {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

#[unsafe(no_mangle)]
#[inline(always)]
extern "C" fn syspspawn(
    name_ptr: *const u8,
    name_len: usize,
    path_ptr: *const u8,
    path_len: usize,
    argv_ptr: *const RawSlice<u8>,
    argv_len: usize,
    flags: SpawnFlags,
    dest_pid: &mut usize,
) -> u16 {
    /// the temporary config struct for the spawn syscall, passed to the syscall
    /// because if it was passed as a bunch of arguments it would be too big to fit
    /// inside the registers
    #[repr(C)]
    struct SpawnConfig {
        name: RawSlice<u8>,
        argv: RawSlice<RawSlice<u8>>,
        flags: SpawnFlags,
    }

    let config = SpawnConfig {
        name: unsafe { RawSlice::from_raw_parts(name_ptr, name_len) },
        argv: unsafe { RawSlice::from_raw_parts(argv_ptr, argv_len) },
        flags,
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
/// # Safety
/// `argv` must be a valid pointer to a slice of slices of `&str`
/// `argv` will become invaild after use, using it is UB
#[inline]
pub unsafe fn unsafe_pspawn(
    name: Option<&str>,
    path: &str,
    argv: *mut [&str],
    flags: SpawnFlags,
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
            argv.as_ptr(),
            argv.len(),
            flags,
            &mut pid,
        ),
        pid
    )
}

/// same as [`pspawn`] but safe because it makes it clear that `argv` is consumed`
#[inline]
pub fn pspawn(
    name: Option<&str>,
    path: &str,
    mut argv: Vec<&str>,
    flags: SpawnFlags,
) -> Result<usize, ErrorStatus> {
    let argv: &mut [&str] = &mut argv;
    unsafe { unsafe_pspawn(name, path, argv as *mut _, flags) }
}

#[inline(always)]
#[unsafe(no_mangle)]
extern "C" fn syswait(pid: usize, exit_code: &mut usize) -> u16 {
    syscall2(SyscallNum::SysWait, pid, exit_code as *mut _ as usize)
}

#[inline]
pub fn wait(pid: usize) -> Result<usize, ErrorStatus> {
    let mut dest_exit_code = 0;
    err_from_u16!(syswait(pid, &mut dest_exit_code), dest_exit_code)
}
