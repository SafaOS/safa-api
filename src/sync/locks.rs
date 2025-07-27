//! Provides various locking mechanisms for synchronization such as Mutex
//!
//! uses Futexes internally

use core::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicU32, Ordering},
    time::Duration,
};

use crate::syscalls::futex::{futex_wait, futex_wake};

const M_AVAILABLE: u32 = 0;
const M_LOCKED: u32 = 1;
const M_WAITED_ON: u32 = 2;

#[must_use = "if unused the Mutex will immediately unlock"]
pub struct MutexGuard<'a, T> {
    mutex: &'a Mutex<T>,
    marker: PhantomData<&'a mut T>,
}

impl<'a, T> Drop for MutexGuard<'a, T> {
    fn drop(&mut self) {
        unsafe {
            self.mutex.force_unlock();
        }
    }
}

impl<'a, T> Deref for MutexGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.mutex.get() }
    }
}

impl<'a, T> DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.mutex.get() }
    }
}

#[derive(Debug)]
pub struct Mutex<T> {
    state: AtomicU32,
    inner: T,
}

impl<T> Mutex<T> {
    /// Constructs a new free Mutex.
    pub const fn new(inner: T) -> Self {
        Self {
            state: AtomicU32::new(M_AVAILABLE),
            inner,
        }
    }
    /// Gets a mutable reference to the inner value.
    pub const fn get_mut(&mut self) -> &mut T {
        &mut self.inner
    }
    /// Gets a mutable pointer to the inner value.
    pub const fn get(&self) -> *mut T {
        &self.inner as *const T as *mut T
    }
    /// Locks the mutex, blocking the current thread until it can be acquired.
    ///
    /// the Mutex is locked until the returned MutexGuard is dropped.
    pub fn lock(&self) -> MutexGuard<'_, T> {
        if let Err(mut s) = self.state.compare_exchange_weak(
            M_AVAILABLE,
            M_LOCKED,
            Ordering::Acquire,
            Ordering::Relaxed,
        ) {
            if s != M_WAITED_ON {
                s = self.state.swap(M_WAITED_ON, Ordering::Acquire);
            }

            while s != M_AVAILABLE {
                futex_wait(&self.state, M_WAITED_ON, Duration::MAX)
                    .expect("System error while waiting for a Futex");

                s = self.state.swap(M_WAITED_ON, Ordering::Acquire);
            }
        }
        MutexGuard {
            mutex: self,
            marker: PhantomData,
        }
    }
    /// Attempts to acquire the mutex without blocking, returning `None` if the mutex is currently locked.
    pub fn try_lock(&self) -> Option<MutexGuard<'_, T>> {
        if self
            .state
            .compare_exchange(
                M_AVAILABLE,
                M_LOCKED,
                core::sync::atomic::Ordering::Acquire,
                core::sync::atomic::Ordering::Relaxed,
            )
            .is_ok()
        {
            Some(MutexGuard {
                mutex: self,
                marker: PhantomData,
            })
        } else {
            None
        }
    }
    /// Forces the mutex to be unlocked, even if it is currently locked.
    pub unsafe fn force_unlock(&self) {
        if self.state.fetch_sub(1, Ordering::Acquire) != M_LOCKED {
            // will also handle the case where the mutex is already unlocked
            self.state.store(M_AVAILABLE, Ordering::Release);
            futex_wake(&self.state, 1).expect("System error while waking 1 Futex");
        }
    }
}

impl<T: Clone> Clone for Mutex<T> {
    fn clone(&self) -> Self {
        Mutex {
            state: AtomicU32::new(M_AVAILABLE),
            inner: self.inner.clone(),
        }
    }
}
