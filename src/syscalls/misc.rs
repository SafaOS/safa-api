use super::{define_syscall, SyscallNum};

define_syscall! {
    SyscallNum::SysShutdown => {
        /// Shuts down the system
        sysshutdown() unreachable
    },
    SyscallNum::SysReboot => {
        /// Reboots the system
        sysreboot() unreachable
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
