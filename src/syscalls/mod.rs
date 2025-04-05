use crate::raw::io::DirEntry;

use super::errors::ErrorStatus;

#[cfg(not(feature = "rustc-dep-of-std"))]
extern crate alloc;
use super::raw::io::FileAttr;
use super::raw::{RawSlice, RawSliceMut};
use core::arch::asm;
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
    ($num: path, $arg1: expr $(,)?) => {
        $crate::syscalls::syscall1($num, $arg1)
    };
    ($num: path, $arg1: expr, $arg2: expr $(,)?) => {
        $crate::syscalls::syscall2($num, $arg1, $arg2)
    };
    ($num: path, $arg1: expr, $arg2: expr, $arg3: expr $(,)?) => {
        $crate::syscalls::syscall3($num, $arg1, $arg2, $arg3)
    };
    ($num: path, $arg1: expr, $arg2: expr, $arg3: expr, $arg4: expr $(,)?) => {
        $crate::syscalls::syscall4($num, $arg1, $arg2, $arg3, $arg4)
    };
    ($num: path, $arg1: expr, $arg2: expr, $arg3: expr, $arg4: expr, $arg5: expr $(,)?) => {
        $crate::syscalls::syscall5($num, $arg1, $arg2, $arg3, $arg4, $arg5)
    };
}

pub(crate) use syscall;

macro_rules! define_syscall {
    ($num:path => { $(#[$($attrss:tt)*])* $name:ident ($($arg:ident : $ty:ty),*) unreachable }) => {
        #[cfg_attr(
            not(any(feature = "std", feature = "rustc-dep-of-std")),
            unsafe(no_mangle)
        )]
        #[inline(always)]
        $(#[$($attrss)*])*
        pub extern "C" fn $name($($arg: $ty),*) -> ! {
            #[allow(unused_imports)]
            use $crate::syscalls::types::IntoSyscallArg;
            _ = $crate::syscalls::syscall!($num, $( $arg.into_syscall_arg() ),*);
            unreachable!()
        }
    };
    ($num:path => { $(#[$($attrss:tt)*])* $name:ident ($($arg:ident : $ty:ty),*) }) => {
        #[cfg_attr(
            not(any(feature = "std", feature = "rustc-dep-of-std")),
            unsafe(no_mangle)
        )]
        #[inline(always)]
        $(#[$($attrss)*])*
        pub extern "C" fn $name($($arg: $ty),*) -> u16 {
            #[allow(unused_imports)]
            use $crate::syscalls::types::IntoSyscallArg;
            let result = $crate::syscalls::syscall!($num, $( $arg.into_syscall_arg() ),*);
            result
        }
    };
    {$($num:path => { $(#[$($attrss:tt)*])* $name:ident ($($arg:ident: $ty:ty),*) $($modifier:tt)* }),*} => {
        $(define_syscall!($num => { $name($($arg: $ty),*) $($modifier)* });)*
    };
}

pub(crate) use define_syscall;

mod raw;
pub use raw::*;
/// Contains documentation-only types for syscall arguments
pub mod types;
