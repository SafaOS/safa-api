//! This module exposes SafaOS's syscalls and their rust counterparts

#[cfg(not(feature = "rustc-dep-of-std"))]
extern crate alloc;
use core::arch::asm;
pub use safa_abi::syscalls::SyscallTable as SyscallNum;

macro_rules! err_from_u16 {
    ($result:expr) => {
        $result.into_result()
    };
    ($result:expr, $ok:expr) => {
        err_from_u16!($result).map(|()| $ok)
    };
}

pub(crate) use err_from_u16;

#[doc(hidden)]
#[inline(always)]
pub fn syscall0<const NUM: u16>() -> SyscallResult {
    let result: u16;
    unsafe {
        #[cfg(target_arch = "x86_64")]
        asm!(
            "int 0x80",
            in("rax") NUM as usize,
            lateout("rax") result,
        );
        #[cfg(target_arch = "aarch64")]
        asm!(
            "svc #{num}",
            num = const NUM,
            lateout("x0") result
        );
        core::mem::transmute(result)
    }
}

#[doc(hidden)]
#[inline(always)]
pub fn syscall1<const NUM: u16>(arg1: usize) -> SyscallResult {
    let result: u16;
    unsafe {
        #[cfg(target_arch = "x86_64")]
        asm!(
            "int 0x80",
            in("rax") NUM as usize,
            in("rdi") arg1,
            lateout("rax") result,
        );
        #[cfg(target_arch = "aarch64")]
        asm!(
            "svc #{num}",
            num = const NUM,
            in("x0") arg1,
            lateout("x0") result
        );
        core::mem::transmute(result)
    }
}

#[doc(hidden)]
#[inline(always)]
pub fn syscall2<const NUM: u16>(arg1: usize, arg2: usize) -> SyscallResult {
    let result: u16;
    unsafe {
        #[cfg(target_arch = "x86_64")]
        asm!(
            "int 0x80",
            in("rax") NUM as usize,
            in("rdi") arg1,
            in("rsi") arg2,
            lateout("rax") result,
        );
        #[cfg(target_arch = "aarch64")]
        asm!(
            "svc #{num}",
            num = const NUM,
            in("x0") arg1,
            in("x1") arg2,
            lateout("x0") result
        );
        core::mem::transmute(result)
    }
}

#[doc(hidden)]
#[inline(always)]
pub fn syscall3<const NUM: u16>(arg1: usize, arg2: usize, arg3: usize) -> SyscallResult {
    let result: u16;
    unsafe {
        #[cfg(target_arch = "x86_64")]
        asm!(
            "int 0x80",
            in("rax") NUM as usize,
            in("rdi") arg1,
            in("rsi") arg2,
            in("rdx") arg3,
            lateout("rax") result,
        );
        #[cfg(target_arch = "aarch64")]
        asm!(
            "svc #{num}",
            num = const NUM,
            in("x0") arg1,
            in("x1") arg2,
            in("x2") arg3,
            lateout("x0") result
        );
        core::mem::transmute(result)
    }
}

#[doc(hidden)]
#[inline(always)]
pub fn syscall4<const NUM: u16>(
    arg1: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
) -> SyscallResult {
    let result: u16;
    unsafe {
        #[cfg(target_arch = "x86_64")]
        asm!(
            "int 0x80",
            in("rax") NUM as usize,
            in("rdi") arg1,
            in("rsi") arg2,
            in("rdx") arg3,
            in("rcx") arg4,
            lateout("rax") result,
        );

        #[cfg(target_arch = "aarch64")]
        asm!(
            "svc #{num}",
            num = const NUM,
            in("x0") arg1,
            in("x1") arg2,
            in("x2") arg3,
            in("x3") arg4,
            lateout("x0") result
        );
        core::mem::transmute(result)
    }
}

#[doc(hidden)]
#[inline(always)]
pub fn syscall5<const NUM: u16>(
    arg1: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
    arg5: usize,
) -> SyscallResult {
    let result: u16;
    unsafe {
        #[cfg(target_arch = "x86_64")]
        asm!(
            "int 0x80",
            in("rax") NUM as usize,
            in("rdi") arg1,
            in("rsi") arg2,
            in("rdx") arg3,
            in("rcx") arg4,
            in("r8") arg5,
            lateout("rax") result,
        );
        #[cfg(target_arch = "aarch64")]
        asm!(
            "svc #{num}",
            num = const NUM,
            in("x0") arg1,
            in("x1") arg2,
            in("x2") arg3,
            in("x3") arg4,
            in("x4") arg5,
            lateout("x0") result
        );
        core::mem::transmute(result)
    }
}

/// Invokes a syscall with the given number and arguments
/// Number must be of type [`SyscallNum`]
/// Arguments must be of type [`usize`]
/// returns a [`SyscallResult`]
macro_rules! syscall {
    ($num: path $(,)?) => {
        $crate::syscalls::syscall0::<{ $num as u16 }>()
    };
    ($num: path, $arg1: expr $(,)?) => {
        $crate::syscalls::syscall1::<{ $num as u16 }>($arg1)
    };
    ($num: path, $arg1: expr, $arg2: expr $(,)?) => {
        $crate::syscalls::syscall2::<{ $num as u16 }>($arg1, $arg2)
    };
    ($num: path, $arg1: expr, $arg2: expr, $arg3: expr $(,)?) => {
        $crate::syscalls::syscall3::<{ $num as u16 }>($arg1, $arg2, $arg3)
    };
    ($num: path, $arg1: expr, $arg2: expr, $arg3: expr, $arg4: expr $(,)?) => {
        $crate::syscalls::syscall4::<{ $num as u16 }>($arg1, $arg2, $arg3, $arg4)
    };
    ($num: path, $arg1: expr, $arg2: expr, $arg3: expr, $arg4: expr, $arg5: expr $(,)?) => {
        $crate::syscalls::syscall5::<{ $num as u16 }>($arg1, $arg2, $arg3, $arg4, $arg5)
    };
}

pub(crate) use syscall;

macro_rules! define_syscall {
    ($num:path => { $(#[$attrss:meta])* $name:ident ($($arg:ident : $ty:ty),*) unreachable }) => {
        $(#[$attrss])*
        #[cfg_attr(
            not(any(feature = "std", feature = "rustc-dep-of-std")),
            unsafe(no_mangle)
        )]
        #[cfg_attr(any(feature = "std", feature = "rustc-dep-of-std"), inline(always))]
        pub extern "C" fn $name($($arg: $ty),*) -> ! {
            #[allow(unused_imports)]
            use $crate::syscalls::types::IntoSyscallArg;
            _ = $crate::syscalls::syscall!($num, $( $arg.into_syscall_arg() ),*);
            unreachable!()
        }
    };
    ($num:path => { $(#[$attrss:meta])* $name:ident ($($arg:ident : $ty:ty),*) }) => {
        $(#[$attrss])*
        #[cfg_attr(
            not(any(feature = "std", feature = "rustc-dep-of-std")),
            unsafe(no_mangle)
        )]
        #[cfg_attr(any(feature = "std", feature = "rustc-dep-of-std"), inline(always))]
        pub extern "C" fn $name($($arg: $ty),*) -> $crate::syscalls::types::SyscallResult {
            #[allow(unused_imports)]
            use $crate::syscalls::types::IntoSyscallArg;
            let result = $crate::syscalls::syscall!($num, $( $arg.into_syscall_arg() ),*);
            result
        }
    };
    {$($num:path => { $(#[$attrss:meta])* $name:ident ($($arg:ident: $ty:ty),*) $($modifier:tt)* }),* $(,)?} => {
        $(define_syscall!($num => { $(#[$attrss])* $name($($arg: $ty),*) $($modifier)* });)*
    };
}

pub(crate) use define_syscall;

/// FS Operations related syscalls (that takes a path) such as create, remove, open, rename and etc
pub mod fs;
/// I/O Operations related syscalls (that takes a resource) such as read, write, truncate, etc
pub mod io;
/// Syscalls and operations that don't fall into a specific category
pub mod misc;
/// (SysP) Process related syscalls and operations
pub mod process;
/// Syscalls and operations related to the current process
pub mod process_misc;
/// (SysR) Resources related syscalls and operations such as destroying resources, duplicating them, etc
pub mod resources;
/// (SysT) Thread related syscalls and operations
pub mod thread;

use types::SyscallResult;
/// Contains documentation-only types for syscall arguments
pub mod types;
