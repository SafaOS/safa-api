//! A high-level API over SafaOS's syscalls
//!
//! for example [`self::alloc`] is a high-level userspace allocator which internally uses the [`self::syscalls::syssbrk`] syscall
//!
//! This crate also exposes raw SafaOS syscalls (see [`self::syscalls`])
//! and raw SafaOS abi structures (see [`self::raw`])

#![cfg_attr(not(feature = "std"), no_std)]

use core::fmt::Write;

use crate::process::stdio::sysget_stderr;

pub mod errors {
    pub use safa_abi::errors::{ErrorStatus, SysResult};

    #[cfg(any(feature = "rustc-dep-of-std", feature = "std"))]
    #[cfg_attr(feature = "rustc-dep-of-std", macro_export)]
    macro_rules! err_from_io_error_kind {
        ($io_err_ty: path, $io_err: ident) => {
            use $crate::errors::ErrorStatus::*;
            use $io_err_ty as IoErrorKind;

            return match $io_err {
                IoErrorKind::NotFound => NoSuchAFileOrDirectory,
                IoErrorKind::AlreadyExists => AlreadyExists,
                IoErrorKind::PermissionDenied => MissingPermissions,
                IoErrorKind::ResourceBusy => Busy,
                IoErrorKind::NotADirectory => NotADirectory,
                IoErrorKind::IsADirectory => NotAFile,
                IoErrorKind::OutOfMemory => OutOfMemory,
                IoErrorKind::Other => Generic,
                IoErrorKind::DirectoryNotEmpty => DirectoryNotEmpty,
                IoErrorKind::Unsupported => OperationNotSupported,

                _ => Generic,
            };
        };
    }

    #[cfg(feature = "std")]
    pub fn err_from_io_error_kind(io_err: std::io::ErrorKind) -> ErrorStatus {
        err_from_io_error_kind!(std::io::ErrorKind, io_err);
    }
}

pub mod alloc;
pub mod process;
pub mod sync;
pub mod syscalls;
pub use safa_abi as abi;
pub use safa_abi::ffi;

#[macro_export]
macro_rules! exported_func {
    {$($meta: meta)? $($inner: tt)*} => {
        $($meta)?
        #[cfg_attr(
            not(any(feature = "std", feature = "rustc-dep-of-std")),
            unsafe(no_mangle)
        )]
        #[cfg_attr(any(feature = "std", feature = "rustc-dep-of-std"), inline(always))]
        $($inner)*
    };
}

#[allow(unused)]
struct Stderr;

fn _print_err(str: &str) {
    let stderr = sysget_stderr();
    _ = syscalls::io::write(stderr, -1, str.as_bytes());
    _ = syscalls::io::sync(stderr);
}
impl Write for Stderr {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        _print_err(s);
        Ok(())
    }
}

#[allow(unused)]
macro_rules! printerr {
    ($($arg:tt)*) => {
        _ = Stderr.write_fmt(format_args!($($arg)*));
    };
}

#[allow(unused)]
macro_rules! printerrln {
    () => {
        printerr!("\n");
    };
    ($($arg:tt)*) => {
        printerr!("{}\n", format_args!($($arg)*));
    };
}

#[cfg(not(any(feature = "std", feature = "rustc-dep-of-std")))]
#[panic_handler]
fn _panic(info: &core::panic::PanicInfo) -> ! {
    printerrln!("Safa-API panicked: {}", info);
    syscalls::process::exit(1);
}
