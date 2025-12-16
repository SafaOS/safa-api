//! contains functions related to standard input/output/error streams descriptors
//! api must be initialized before using these functions, see [`super::init`]

use core::{cell::UnsafeCell, mem::MaybeUninit};

use crate::{
    exported_func,
    syscalls::{self, types::Ri},
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

#[cfg_attr(feature = "linkonce", unsafe(no_mangle))]
#[cfg_attr(feature = "linkonce", linkage = "linkonce")]
pub(super) static SAAPI_ABI_STRUCTURES: StaticAbiStructures =
    StaticAbiStructures(UnsafeCell::new(MaybeUninit::zeroed()));

static STDIO: LazyCell<ProcessStdio> =
    LazyCell::new(|| unsafe { SAAPI_ABI_STRUCTURES.get().stdio });
static STDIN: LazyCell<Ri> = LazyCell::new(|| {
    let stdin: Option<Ri> = STDIO.into_rust().1;
    if let Some(stdin) = stdin {
        stdin
    } else {
        syscalls::fs::open_all("dev:/tty").expect("failed to fall back to `dev:/tty` for stdin")
    }
});

static STDOUT: LazyCell<Ri> = LazyCell::new(|| {
    let stdout: Option<Ri> = STDIO.into_rust().0;
    if let Some(stdout) = stdout {
        stdout
    } else {
        syscalls::fs::open_all("dev:/tty").expect("failed to fall back to `dev:/tty` for stdout")
    }
});

static STDERR: LazyCell<Ri> = LazyCell::new(|| {
    let stderr: Option<Ri> = STDIO.into_rust().2;
    if let Some(stderr) = stderr {
        stderr
    } else {
        syscalls::fs::open_all("dev:/tty").expect("failed to fall back to `dev:/tty` for stderr")
    }
});

exported_func! {
    /// Returns the resource id of the stdout file descriptor (if available)
    pub extern "C" fn systry_get_stdout() -> COption<Ri> {
        STDIO.into_rust().0.into()
    }
}

exported_func! {
    /// Returns the resource id of the stderr file descriptor (if available)
    pub extern "C" fn systry_get_stderr() -> COption<Ri> {
        STDIO.into_rust().2.into()
    }
}

exported_func! {
    /// Returns the resource id of the stdin file descriptor (if available)
    pub extern "C" fn systry_get_stdin() -> COption<Ri> {
        STDIO.into_rust().1.into()
    }
}

exported_func! {
    /// Returns the resource id of the stdout file descriptor
    ///
    /// if there is no stdout file descriptor, it will fall back to `dev:/tty`
    pub extern "C" fn sysget_stdout() -> Ri {
        *STDOUT
    }
}

exported_func! {
    /// Returns the resource id of the stderr file descriptor
    ///
    /// if there is no stderr file descriptor, it will fall back to `dev:/tty`
    pub extern "C" fn sysget_stderr() -> Ri {
        *STDERR
    }
}

exported_func! {
    /// Returns the resource id of the stdin file descriptor
    ///
    /// if there is no stdin file descriptor, it will fall back to `dev:/tty`
    pub extern "C" fn sysget_stdin() -> Ri {
        *STDIN
    }
}

pub fn init_meta(abi_structures: AbiStructures) {
    unsafe { SAAPI_ABI_STRUCTURES.init(abi_structures) };
}
