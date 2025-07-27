//! contains functions related to standard input/output/error streams descriptors
//! api must be initialized before using these functions, see [`super::init`]

use core::{cell::UnsafeCell, mem::MaybeUninit};

use crate::{
    exported_func,
    syscalls::{self},
};
use safa_abi::{
    ffi::option::COption,
    process::{AbiStructures, ProcessStdio},
};

use crate::sync::cell::LazyCell;

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

static STDIO: LazyCell<ProcessStdio> = LazyCell::new(|| unsafe { ABI_STRUCTURES.get().stdio });
static STDIN: LazyCell<usize> = LazyCell::new(|| {
    let stdin: Option<usize> = STDIO.stdin.into();
    if let Some(stdin) = stdin {
        stdin
    } else {
        syscalls::fs::open_all("dev:/tty").expect("failed to fall back to `dev:/tty` for stdin")
    }
});

static STDOUT: LazyCell<usize> = LazyCell::new(|| {
    let stdout: Option<usize> = STDIO.stdout.into();
    if let Some(stdout) = stdout {
        stdout
    } else {
        syscalls::fs::open_all("dev:/tty").expect("failed to fall back to `dev:/tty` for stdout")
    }
});

static STDERR: LazyCell<usize> = LazyCell::new(|| {
    let stderr: Option<usize> = STDIO.stderr.into();
    if let Some(stderr) = stderr {
        stderr
    } else {
        syscalls::fs::open_all("dev:/tty").expect("failed to fall back to `dev:/tty` for stderr")
    }
});

exported_func! {
    /// Returns the resource id of the stdout file descriptor (if available)
    pub extern "C" fn systry_get_stdout() -> COption<usize> {
        STDIO.stdout.clone()
    }
}

exported_func! {
    /// Returns the resource id of the stderr file descriptor (if available)
    pub extern "C" fn systry_get_stderr() -> COption<usize> {
        STDIO.stderr.clone()
    }
}

exported_func! {
    /// Returns the resource id of the stdin file descriptor (if available)
    pub extern "C" fn systry_get_stdin() -> COption<usize> {
        STDIO.stdin.clone()
    }
}

exported_func! {
    /// Returns the resource id of the stdout file descriptor
    ///
    /// if there is no stdout file descriptor, it will fall back to `dev:/tty`
    pub extern "C" fn sysget_stdout() -> usize {
        *STDOUT
    }
}

exported_func! {
    /// Returns the resource id of the stderr file descriptor
    ///
    /// if there is no stderr file descriptor, it will fall back to `dev:/tty`
    pub extern "C" fn sysget_stderr() -> usize {
        *STDERR
    }
}

exported_func! {
    /// Returns the resource id of the stdin file descriptor
    ///
    /// if there is no stdin file descriptor, it will fall back to `dev:/tty`
    pub extern "C" fn sysget_stdin() -> usize {
        *STDIN
    }
}

pub fn init_meta(abi_structures: AbiStructures) {
    unsafe { ABI_STRUCTURES.init(abi_structures) };
}
