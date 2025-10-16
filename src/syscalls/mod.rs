//! This module exposes SafaOS's syscalls and their rust counterparts

#[cfg(not(feature = "rustc-dep-of-std"))]
extern crate alloc;

pub(crate) mod call;

pub use safa_abi::syscalls::SyscallTable as SyscallNum;

pub use call::syscall;

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
            let _: $crate::syscalls::types::SyscallResults<core::convert::Infallible> = $crate::syscalls::syscall!($num, $( $arg.into_syscall_arg() ),*);
            unreachable!()
        }
    };
    ($num:path => { $(#[$attrss:meta])* $name:ident ($($arg:ident : $ty:ty),*) $($return_ty:ty)? }) => {
        $(#[$attrss])*
        #[cfg_attr(
            not(any(feature = "std", feature = "rustc-dep-of-std")),
            unsafe(no_mangle)
        )]
        #[cfg_attr(any(feature = "std", feature = "rustc-dep-of-std"), inline(always))]
        pub extern "C" fn $name($($arg: $ty),*) -> $crate::syscalls::types::SyscallResults$(<$return_ty>)? {
            #[allow(unused_imports)]
            use $crate::syscalls::types::IntoSyscallArg;
            let result = $crate::syscalls::syscall!($num, $( $arg ),*);
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
/// (SysTFut) Futex related syscalls and operations
pub mod futex;
/// I/O Operations related syscalls (that takes a resource) such as read, write, truncate, etc
pub mod io;
/// (SysMem) Memory related syscalls
pub mod mem;
/// Syscalls and operations that don't fall into a specific category
pub mod misc;
/// (SysP) Process related syscalls and operations
pub mod process;
/// Syscalls and operations related to the current process
pub mod process_misc;
/// (SysR) Resources related syscalls and operations such as destroying resources, duplicating them, etc
pub mod resources;
/// (SysSock) Unix Sockets related syscalls and operations
pub mod sockets;
/// (SysT) Thread related syscalls and operations
pub mod thread;
/// Contains documentation-only types for syscall arguments
pub mod types;
