//! A high-level API over SafaOS's syscalls
//!
//! for example [`self::alloc`] is a high-level userspace allocator which internally uses the [`self::syscalls::syssbrk`] syscall
//!
//! This crate also exposes raw SafaOS syscalls (see [`self::syscalls`])
//! and raw SafaOS abi structures (see [`self::raw`])

#![cfg_attr(not(feature = "std"), no_std)]

use core::{cell::LazyCell, fmt::Write, ops::Deref, ptr::NonNull};

use alloc::GLOBAL_SYSTEM_ALLOCATOR;
use process::sysmeta_stderr;

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
pub mod raw;
pub mod syscalls;

// FIXME: introduce locks when threads are added
pub(crate) struct Lazy<T>(core::cell::LazyCell<T>);
impl<T> Lazy<T> {
    pub const fn new(value: fn() -> T) -> Self {
        Self(core::cell::LazyCell::new(value))
    }
}

impl<T> Deref for Lazy<T> {
    type Target = LazyCell<T>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

unsafe impl<T> Sync for Lazy<T> {}
unsafe impl<T> Send for Lazy<T> {}

#[allow(unused)]
struct Stderr;

fn _print_err(str: &str) {
    let stderr = sysmeta_stderr();
    _ = syscalls::write(stderr, -1, str.as_bytes());
    _ = syscalls::sync(stderr);
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
    printerrln!("Safa-API panicked: {}", info.message(),);
    if let Some(location) = info.location() {
        printerrln!("at {}", location);
    }
    syscalls::exit(1);
}

/// Converts argv to a CStr and calls `main` with the new argv
/// and exits with the result
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _c_start_inner(
    argc: usize,
    argv: *mut (NonNull<u8>, usize),
    main: extern "C" fn(i32, *const NonNull<u8>) -> i32,
) -> ! {
    let argv_slice = unsafe { core::slice::from_raw_parts(argv, argc) };
    let bytes = argc * size_of::<usize>();

    let c_argv_bytes = GLOBAL_SYSTEM_ALLOCATOR.allocate(bytes).unwrap();
    let c_argv_slice =
        unsafe { core::slice::from_raw_parts_mut(c_argv_bytes.as_ptr() as *mut NonNull<u8>, argc) };

    for (i, (arg_ptr, _)) in argv_slice.iter().enumerate() {
        c_argv_slice[i] = *arg_ptr;
    }

    let result = main(argc as i32, c_argv_slice.as_ptr());
    syscalls::exit(result as usize)
}
