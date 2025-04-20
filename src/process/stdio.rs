//! contains functions related to standard input/output/error streams descriptors
//! api must be initialized before using these functions, see [`super::init`]

use core::{cell::UnsafeCell, mem::MaybeUninit};

use crate::syscalls::{self};
use safa_abi::raw::processes::{AbiStructures, TaskStdio};

use crate::Lazy;

pub(super) struct StaticAbiStructures(UnsafeCell<MaybeUninit<AbiStructures>>);

impl StaticAbiStructures {
    pub unsafe fn init(&self, structures: AbiStructures) {
        let ptr = self.0.get();
        ptr.write(MaybeUninit::new(structures));
    }

    unsafe fn get(&'static self) -> &'static AbiStructures {
        let ptr = self.0.get();
        MaybeUninit::assume_init_ref(&*ptr)
    }
}

unsafe impl Sync for StaticAbiStructures {}

pub(super) static ABI_STRUCTURES: StaticAbiStructures =
    StaticAbiStructures(UnsafeCell::new(MaybeUninit::zeroed()));

static STDIO: Lazy<TaskStdio> = Lazy::new(|| unsafe { ABI_STRUCTURES.get().stdio });
static STDIN: Lazy<usize> = Lazy::new(|| {
    let stdin: Option<usize> = STDIO.stdin.into();
    if let Some(stdin) = stdin {
        stdin
    } else {
        syscalls::open("dev:/tty").expect("failed to fall back to `dev:/tty` for stdin")
    }
});

static STDOUT: Lazy<usize> = Lazy::new(|| {
    let stdout: Option<usize> = STDIO.stdout.into();
    if let Some(stdout) = stdout {
        stdout
    } else {
        syscalls::open("dev:/tty").expect("failed to fall back to `dev:/tty` for stdout")
    }
});

static STDERR: Lazy<usize> = Lazy::new(|| {
    let stderr: Option<usize> = STDIO.stderr.into();
    if let Some(stderr) = stderr {
        stderr
    } else {
        syscalls::open("dev:/tty").expect("failed to fall back to `dev:/tty` for stderr")
    }
});

/// Returns the resource id of the stdout file descriptor
///
/// if there is no stdout file descriptor, it will fall back to `dev:/tty`
#[cfg_attr(
    not(any(feature = "std", feature = "rustc-dep-of-std")),
    unsafe(no_mangle)
)]
#[inline(always)]
pub extern "C" fn sysmeta_stdout() -> usize {
    **STDOUT
}

/// Returns the resource id of the stderr file descriptor
///
/// if there is no stderr file descriptor, it will fall back to `dev:/tty`
#[cfg_attr(
    not(any(feature = "std", feature = "rustc-dep-of-std")),
    unsafe(no_mangle)
)]
#[inline(always)]
pub extern "C" fn sysmeta_stderr() -> usize {
    **STDERR
}

/// Returns the resource id of the stdin file descriptor
///
/// if there is no stdin file descriptor, it will fall back to `dev:/tty`
#[cfg_attr(
    not(any(feature = "std", feature = "rustc-dep-of-std")),
    unsafe(no_mangle)
)]
#[inline(always)]
pub extern "C" fn sysmeta_stdin() -> usize {
    **STDIN
}

pub fn init_meta(abi_structures: AbiStructures) {
    unsafe { ABI_STRUCTURES.init(abi_structures) };
}
