use core::time::Duration;

use safa_abi::errors::ErrorStatus;

use crate::syscalls::types::{OptionalPtrMut, RequiredPtrMut};

use super::define_syscall;
use super::err_from_u16;
use super::SyscallNum;

define_syscall! {
    SyscallNum::SysTFutWake => {
        /// Wakes up, up to `n` threads waiting on futex `addr` using [`syst_fut_wait`]
        ///
        /// puts the amount of threads that were woken up into `wake_results`
        syst_fut_wake(addr: RequiredPtrMut<u32>, n: usize, wake_results: OptionalPtrMut<usize>)
    },
    SyscallNum::SysTFutWait => {
        /// Waits for *addr to not be equal to val
        /// only stops waiting if *addr != val and signaled by [`syst_fut_wake`] or timeout is reached
        ///
        /// `wait_results` is going to be set to true if *addr != val, false if timeout is reached
        syst_fut_wait(addr: RequiredPtrMut<u32>, val: u32, timeout_ms: u64, wait_results: OptionalPtrMut<bool>)
    }
}

/// Wakes up, up to `n` threads waiting on futex `addr` using [`futex_wait`]
///
/// returns the amount of threads that were woken up on success
/// # Safety
/// This function is safe because the value at `addr` is not accessed unless there were another thread waiting on it using `futex_wait`
#[inline]
pub fn futex_wake(addr: *mut u32, n: usize) -> Result<usize, ErrorStatus> {
    let mut results = 0;

    let results_ptr = RequiredPtrMut::new(&mut results).into();
    let addr = unsafe { RequiredPtrMut::new_unchecked(addr) };

    err_from_u16!(syst_fut_wake(addr, n, results_ptr), results)
}

/// Waits for *addr to not be equal to val
/// only stops waiting if *addr != val and signaled by [`futex_wake`] or timeout is reached
///
/// Returns true if *addr != val, false if timeout is reached
/// # Safety
/// unsafe because `addr` must be a valid pointer to a u32 value that is alive until at least timeout is reached
#[inline]
pub unsafe fn futex_wait(
    addr: *mut u32,
    val: u32,
    timeout_duration: Duration,
) -> Result<bool, ErrorStatus> {
    assert!(addr.is_aligned());
    assert!(!addr.is_null());

    let timeout_ms = timeout_duration.as_millis() as u64;

    let mut results = false;
    let results_ptr = RequiredPtrMut::new(&mut results).into();
    let addr = unsafe { RequiredPtrMut::new_unchecked(addr) };

    err_from_u16!(syst_fut_wait(addr, val, timeout_ms, results_ptr), results)
}
