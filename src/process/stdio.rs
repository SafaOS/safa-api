//! contains functions related to standard input/output/error streams descriptors
//! api must be initialized before using these functions, see [`super::init`]

use crate::{
    exported_func,
    process::proc_meta,
    syscalls::{self, types::Ri},
};
use safa_abi::{ffi::option::COption, process::ProcessStdio};

use crate::sync::lazy::LazyLock;

static STDIO: LazyLock<ProcessStdio> = LazyLock::new(|| proc_meta().stdio);
static STDIN: LazyLock<Ri> = LazyLock::new(|| {
    let stdin: Option<Ri> = STDIO.into_rust().1;
    if let Some(stdin) = stdin {
        stdin
    } else {
        syscalls::fs::open_all("dev:/tty").expect("failed to fall back to `dev:/tty` for stdin")
    }
});

static STDOUT: LazyLock<Ri> = LazyLock::new(|| {
    let stdout: Option<Ri> = STDIO.into_rust().0;
    if let Some(stdout) = stdout {
        stdout
    } else {
        syscalls::fs::open_all("dev:/tty").expect("failed to fall back to `dev:/tty` for stdout")
    }
});

static STDERR: LazyLock<Ri> = LazyLock::new(|| {
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
