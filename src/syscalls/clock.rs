use core::time::Duration;

use crate::syscalls::{
    types::{IntoSyscallArg, RequiredPtr, RequiredPtrMut},
    SyscallNum,
};
pub use safa_abi::clock::*;
use safa_abi::errors::ErrorStatus;

impl IntoSyscallArg for Clock {
    type RegResults = <u32 as IntoSyscallArg>::RegResults;
    fn into_syscall_arg(self) -> Self::RegResults {
        <u32 as IntoSyscallArg>::into_syscall_arg(self as u32)
    }
}

define_syscall! {
    SyscallNum::SysUptime => {
        /// returns the system uptime in milliseconds
        sysuptime(uptime: RequiredPtrMut<u64>)
    },
    SyscallNum::SysClockGetTime => {
        /// Gets the [`core::time::Duration`] that has passed since a given [`Clock`].
        sysclock_gettime(clock: Clock, results: RequiredPtrMut<CDuration>)
    },
    SyscallNum::SysClockGetRes => {
        /// Gets the smallest [`core::time::Duration`] that a given [`Clock`] can produce. (clock resolution/precision).
        sysclock_getres(clock: Clock, results: RequiredPtrMut<CDuration>)
    },
    SyscallNum::SysClockSetTime => {
        /// Sets the time to the given `time` in a given [`Clock`].
        /// Depending on the clock this might fail/require privileges.
        sysclock_settime(clock: Clock, time: RequiredPtr<CDuration>)
    },
    SyscallNum::SysClockGetCntFreq => {
        /// Gets the frequency of the hardware counter (such as the TSC on x86_64).
        sysclock_getcntfreq(results: RequiredPtrMut<u64>, flags: u32)
    }
}

#[inline]
#[deprecated(
    since = "0.6.1",
    note = "Please use `clock_gettime(Clock::Monotonic, ...)` instead"
)]
pub fn uptime() -> u64 {
    let mut results: u64 = 0;
    let ptr = unsafe { RequiredPtrMut::new_unchecked(&raw mut results) };
    sysuptime(ptr);
    results
}

/// Gets the frequency of the hardware counter (such as the TSC on x86_64) in hz.
#[inline]
pub fn getcntfreq() -> u64 {
    let mut results: u64 = 0;
    let ptr = unsafe { RequiredPtrMut::new_unchecked(&raw mut results) };
    sysclock_getcntfreq(ptr, 0);
    results
}

#[inline]
/// Gets the value of the hardware counter.
///
/// See [`getcntfreq`].
pub fn getcnt() -> u64 {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::x86_64::_mm_lfence();
        core::arch::x86_64::_rdtsc()
    }
    #[cfg(target_arch = "aarch64")]
    unsafe {
        core::arch::asm!("isb sy");
        let val: u64;
        core::arch::asm!("mrs {}, cntvct_el0", out(reg) val);
        val
    }
}

/// Calls [`getcnt`] but converts the cnt to a duration given a freq.
/// get the freq with [`getcntfreq`].
#[inline]
pub fn getcnttime(freq: u64) -> Duration {
    let cnt = getcnt();
    // (cnt / freq)*1_000_000_000
    let nanos = (cnt as u128)
        .saturating_mul(1_000_000_000)
        .checked_div(freq as u128)
        .unwrap_or(0);
    Duration::from_nanos(nanos as u64)
}

/// Gets the [`core::time::Duration`] that has passed since a given [`Clock`].
#[inline]
pub fn clock_gettime(clock: Clock) -> Duration {
    let mut results: CDuration = CDuration::ZERO;
    let ptr = unsafe { RequiredPtrMut::new_unchecked(&raw mut results) };
    sysclock_gettime(clock, ptr);

    results.into()
}

#[inline]
/// Sets the time to the given `time` in a given [`Clock`].
/// Depending on the clock this might fail/require privileges.
pub fn clock_settime(clock: Clock, time: Duration) -> Result<(), ErrorStatus> {
    let time: CDuration = time.into();
    let ptr = unsafe { RequiredPtr::new_unchecked((&raw const time).cast_mut()) };
    sysclock_settime(clock, ptr).get()?;
    Ok(())
}

/// Gets the smallest [`core::time::Duration`] that a given [`Clock`] can produce. (clock resolution/precision).
#[inline]
pub fn clock_getres(clock: Clock) -> Duration {
    let mut results: CDuration = CDuration::ZERO;
    let ptr = unsafe { RequiredPtrMut::new_unchecked(&raw mut results) };
    sysclock_getres(clock, ptr);

    results.into()
}
