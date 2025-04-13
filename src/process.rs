//! Module for process-related high-level functions over process related syscalls
//!
//! Such as api initialization functions [`_c_api_init`], environment variables, and process arguments

use core::ptr::NonNull;

use crate::{
    alloc::GLOBAL_SYSTEM_ALLOCATOR,
    syscalls::{self, define_syscall},
};
use safa_abi::{errors::ErrorStatus, raw::processes::TaskMetadata};

use crate::{syscalls::err_from_u16, syscalls::SyscallNum, Lazy};

static META: Lazy<TaskMetadata> =
    Lazy::new(|| meta_take().expect("failed to take ownership of the task metadata"));

// PEAK design

static STDIN: Lazy<usize> = Lazy::new(|| {
    let stdin: Option<usize> = META.stdin.into();
    if let Some(stdin) = stdin {
        stdin
    } else {
        syscalls::open("dev:/tty").expect("failed to fall back to `dev:/tty` for stdin")
    }
});

static STDOUT: Lazy<usize> = Lazy::new(|| {
    let stdout: Option<usize> = META.stdout.into();
    if let Some(stdout) = stdout {
        stdout
    } else {
        syscalls::open("dev:/tty").expect("failed to fall back to `dev:/tty` for stdout")
    }
});

use syscalls::types::SyscallResult;
static STDERR: Lazy<usize> = Lazy::new(|| {
    let stderr: Option<usize> = META.stderr.into();
    if let Some(stderr) = stderr {
        stderr
    } else {
        syscalls::open("dev:/tty").expect("failed to fall back to `dev:/tty` for stderr")
    }
});

define_syscall!(SyscallNum::SysMetaTake => {
    /// Takes ownership of the task metadata
    /// the task metadata is used to store the stdin, stdout, and stderr file descriptors
    /// this syscall can only be called once otherwise it will return [`ErrorStatus::Generic`] (1)
    sysmeta_take(dest_task: *mut TaskMetadata)
});

#[inline]
pub fn meta_take() -> Result<TaskMetadata, ErrorStatus> {
    let mut dest_meta: TaskMetadata = unsafe { core::mem::zeroed() };
    err_from_u16!(sysmeta_take(&raw mut dest_meta), dest_meta)
}

/// Returns the resource id of the stdout file descriptor
#[cfg_attr(
    not(any(feature = "std", feature = "rustc-dep-of-std")),
    unsafe(no_mangle)
)]
#[inline(always)]
pub extern "C" fn sysmeta_stdout() -> usize {
    **STDOUT
}

/// Returns the resource id of the stderr file descriptor
#[cfg_attr(
    not(any(feature = "std", feature = "rustc-dep-of-std")),
    unsafe(no_mangle)
)]
#[inline(always)]
pub extern "C" fn sysmeta_stderr() -> usize {
    **STDERR
}

/// Returns the resource id of the stdin file descriptor
#[cfg_attr(
    not(any(feature = "std", feature = "rustc-dep-of-std")),
    unsafe(no_mangle)
)]
#[inline(always)]
pub extern "C" fn sysmeta_stdin() -> usize {
    **STDIN
}

/// Initializes the safa-api, calls `main`, and exits with the result
/// main are designed as C main function,
///
/// this function is designed to be called from C code at _start before main and main should be passed as a parameter
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _c_api_init(
    argc: usize,
    argv: *mut (NonNull<u8>, usize),
    main: extern "C" fn(argc: i32, argv: *const NonNull<u8>) -> i32,
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
