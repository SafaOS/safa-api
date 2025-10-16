use core::sync::atomic::AtomicU32;
use core::time::Duration;

use safa_abi::errors::ErrorStatus;

use crate::syscalls::types::RequiredPtr;
use crate::syscalls::types::RequiredPtrMut;

use super::define_syscall;
use super::SyscallNum;

define_syscall! {
    SyscallNum::SysTFutWake => {
        /// Wakes up, up to `n` threads waiting on futex `addr` using [`syst_fut_wait`]
        ///
        /// returns the amount of threads that were woken up on success.
        syst_fut_wake(addr: RequiredPtr<AtomicU32>, n: usize) usize
    },
    SyscallNum::SysTFutWait => {
        /// Waits for *addr to not be equal to val
        /// only stops waiting if *addr != val and signaled by [`syst_fut_wake`] or timeout is reached
        ///
        /// if timeout is reached returns [`ErrorStatus::Timeout`]
        syst_fut_wait(addr: RequiredPtr<AtomicU32>, val: u32, timeout_ms: u64)
    }
}

/// Wakes up, up to `n` threads waiting on futex `addr` using [`futex_wait`]
///
/// returns the amount of threads that were woken up on success
/// # Safety
/// This function is safe because the value at `addr` is not accessed unless there were another thread waiting on it using `futex_wait`
#[inline]
pub fn futex_wake(addr: &AtomicU32, n: usize) -> Result<usize, ErrorStatus> {
    let addr = unsafe { RequiredPtr::new_unchecked(addr as *const _ as *mut _) };
    syst_fut_wake(addr, n).get()
}

/// Waits for *addr to not be equal to val
/// only stops waiting if *addr != val and signaled by [`futex_wake`] or timeout is reached
///
/// Returns [`ErrorStatus::Timeout`] if timeout is reached.
#[inline]
pub fn futex_wait(
    addr: &AtomicU32,
    val: u32,
    timeout_duration: Duration,
) -> Result<(), ErrorStatus> {
    let timeout_ms = timeout_duration.as_millis() as u64;
    let addr = unsafe { RequiredPtrMut::new_unchecked(addr as *const _ as *mut _) };

    syst_fut_wait(addr, val, timeout_ms).get()
}
