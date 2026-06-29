use safa_abi::arch::ArchOp;

use crate::syscalls::types::IntoSyscallArg;

use super::{define_syscall, SyscallNum};

impl IntoSyscallArg for ArchOp {
    type RegResults = <u32 as IntoSyscallArg>::RegResults;
    fn into_syscall_arg(self) -> Self::RegResults {
        (self as u32).into_syscall_arg()
    }
}

define_syscall! {
    SyscallNum::SysShutdown => {
        /// Shuts down the system
        sysshutdown() unreachable
    },
    SyscallNum::SysReboot => {
        /// Reboots the system
        sysreboot() unreachable
    },
    SyscallNum::SysACtrl => {
        sysarch_ctrl(op: ArchOp, arg: u64)
    }
}

#[inline]
pub fn shutdown() -> ! {
    sysshutdown()
}

#[inline]
pub fn reboot() -> ! {
    sysreboot()
}

/// uptime and sysuptime are really just deprecated misc syscalls.
#[allow(deprecated)]
pub use super::clock::{sysuptime, uptime};
