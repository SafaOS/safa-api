use crate::syscalls::{self, define_syscall};
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
