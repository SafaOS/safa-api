#![cfg_attr(not(feature = "std"), no_std)]

pub mod errors {
    pub use safa_utils::errors::{ErrorStatus, SysResult};

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
pub mod raw;
pub mod syscalls;
