//! A high-level API over SafaOS's syscalls
//!
//! for example [`self::alloc`] is a high-level userspace allocator which internally uses the [`self::syscalls::syssbrk`] syscall
//!
//! This crate also exposes raw SafaOS syscalls (see [`self::syscalls`])
//! and raw SafaOS abi structures (see [`self::raw`])

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(feature = "linkonce", feature(linkage))]

use core::fmt::{Arguments, Write};

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
                IoErrorKind::HostUnreachable => HostUnreachable,
                IoErrorKind::NetworkUnreachable => NetworkUnreachable,
                IoErrorKind::AddrNotAvailable => AddressNotFound,
                IoErrorKind::AddrInUse => AddressAlreadyInUse,
                IoErrorKind::ConnectionRefused => ConnectionRefused,
                IoErrorKind::TimedOut => Timeout,
                IoErrorKind::ConnectionReset => ConnectionClosed,

                _ => Generic,
            };
        };
    }

    #[cfg(feature = "std")]
    pub fn err_from_io_error_kind(io_err: std::io::ErrorKind) -> ErrorStatus {
        err_from_io_error_kind!(std::io::ErrorKind, io_err);
    }

    #[cfg(any(feature = "rustc-dep-of-std", feature = "std"))]
    #[cfg_attr(feature = "rustc-dep-of-std", macro_export)]
    macro_rules! err_into_io_error_kind {
        ($err: ident, $io_err_ty: path) => {
            use $crate::errors::ErrorStatus::*;
            use $io_err_ty as IoErrorKind;

            #[cfg(feature = "std")]
            const fn unknown_err() -> IoErrorKind {
                IoErrorKind::Other
            }

            #[cfg(feature = "rustc-dep-of-std")]
            const fn unknown_err() -> IoErrorKind {
                IoErrorKind::Uncategorized
            }

            return match $err {
                NoSuchAFileOrDirectory => IoErrorKind::NotFound,
                AlreadyExists => IoErrorKind::AlreadyExists,
                MissingPermissions => IoErrorKind::PermissionDenied,
                Busy => IoErrorKind::ResourceBusy,
                NotADirectory => IoErrorKind::NotADirectory,
                NotAFile => IoErrorKind::IsADirectory,
                NotADevice => IoErrorKind::Unsupported,
                InvalidPath | InvalidPid | InvalidTid | UnknownResource | UnsupportedResource
                | InvalidOffset | InvalidPtr | StrTooLong | TooShort | InvalidSize => {
                    IoErrorKind::InvalidInput
                }
                InvalidStr | Corrupted | NotExecutable | TypeMismatch => IoErrorKind::InvalidData,
                OutOfMemory => IoErrorKind::OutOfMemory,
                DirectoryNotEmpty => IoErrorKind::DirectoryNotEmpty,
                OperationNotSupported | NotSupported | InvalidSyscall | ProtocolNotSupported => {
                    IoErrorKind::Unsupported
                }
                NotEnoughArguments | Generic | MMapError | Panic | Unknown
                | ResourceCloneFailed | NotBound => unknown_err(),
                InvalidArgument | InvalidCommand => IoErrorKind::InvalidInput,
                Timeout => IoErrorKind::TimedOut,
                ConnectionClosed => IoErrorKind::ConnectionReset,
                ConnectionRefused => IoErrorKind::ConnectionRefused,
                AddressNotFound => IoErrorKind::AddrNotAvailable,
                WouldBlock => IoErrorKind::WouldBlock,
                ForceTerminated => IoErrorKind::Interrupted,
                AddressAlreadyInUse => IoErrorKind::AddrInUse,
                NetworkUnreachable => IoErrorKind::NetworkUnreachable,
                HostUnreachable => IoErrorKind::HostUnreachable,
            };
        };
    }

    #[cfg(feature = "std")]
    pub fn err_into_io_error_kind(err: ErrorStatus) -> std::io::ErrorKind {
        err_into_io_error_kind!(err, std::io::ErrorKind);
    }

    #[cfg(feature = "std")]
    pub fn into_io_error(err: ErrorStatus) -> std::io::Error {
        let kind = err_into_io_error_kind(err);
        std::io::Error::new(kind, err.as_str())
    }
}

pub mod alloc;
pub mod net;
pub mod process;
/// An interface over SafaOS's Unix Sockets
pub mod sockets;
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

#[doc(hidden)]
pub fn _write_stderr(args: Arguments) {
    _ = Stderr.write_fmt(args);
}

#[macro_export]
#[allow(unused)]
macro_rules! printerr {
    ($($arg:tt)*) => {
        $crate::_write_stderr(format_args!($($arg)*));
    };
}

#[macro_export]
#[allow(unused)]
macro_rules! printerrln {
    () => {
        $crate::printerr!("\n");
    };
    ($($arg:tt)*) => {
        $crate::printerr!("{}\n", format_args!($($arg)*));
    };
}

#[cfg(not(any(feature = "std", feature = "rustc-dep-of-std")))]
#[panic_handler]
fn _panic(info: &core::panic::PanicInfo) -> ! {
    printerrln!("Safa-API panicked: {}", info);
    syscalls::process::exit(1);
}
