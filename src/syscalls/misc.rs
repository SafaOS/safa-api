use crate::syscalls::types::RequiredPtrMut;

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

define_syscall! {
    SyscallNum::SysUptime => {
        /// returns the system uptime in milliseconds
        sysuptime(uptime: RequiredPtrMut<u64>)
    }
}

#[inline]
pub fn uptime() -> u64 {
    let mut results: u64 = 0;
    let ptr = unsafe { RequiredPtrMut::new_unchecked(&raw mut results) };
    sysuptime(ptr);
    results
}
