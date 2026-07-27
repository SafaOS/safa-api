//! Copied from rust's port futexes's.
use core::sync::atomic::AtomicU32;
use core::time::Duration;

/// An atomic for use as a futex that is at least 32-bits but may be larger
pub type Futex = AtomicU32;
/// Must be the underlying type of Futex
pub type Primitive = u32;

/// An atomic for use as a futex that is at least 8-bits but may be larger.
pub type SmallFutex = AtomicU32;
/// Must be the underlying type of SmallFutex
pub type SmallPrimitive = u32;

pub fn futex_wait(futex: &AtomicU32, expected: u32, timeout: Option<Duration>) -> bool {
    // FIXME: Infinite timeout is just the max for now
    let timeout_duration = timeout.unwrap_or(Duration::MAX);
    let results = crate::syscalls::futex::futex_wait(futex, expected, timeout_duration);
    let timedout = results == Err(crate::errors::ErrorStatus::Timeout);
    if let Err(err) = results {
        if !timedout {
            panic!(
                "FATAL System error while waiting for Futex: {}",
                err.as_str()
            );
        }
    }
    !timedout
}

#[inline]
pub fn futex_wake(futex: &AtomicU32) -> bool {
    let results = crate::syscalls::futex::futex_wake(futex, 1);
    let timedout = results == Err(crate::errors::ErrorStatus::Timeout);
    if let Err(err) = results {
        panic!("FATAL System error while waking Futex: {}", err.as_str());
    }
    !timedout
}

#[inline]
pub fn futex_wake_all(futex: &AtomicU32) {
    crate::syscalls::futex::futex_wake(futex, usize::MAX)
        .expect("FATAL System error while waking all Futexes");
}
