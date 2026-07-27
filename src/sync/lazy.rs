use super::once::OnceExclusiveState;
use crate::sync::Once;
use core::cell::UnsafeCell;
use core::mem::ManuallyDrop;
use core::ops::{Deref, DerefMut};
use core::{fmt, ptr};

// We use the state of a Once as discriminant value. Upon creation, the state is
// "incomplete" and `f` contains the initialization closure. In the first call to
// `call_once`, `f` is taken and run. If it succeeds, `value` is set and the state
// is changed to "complete". If it panics, the Once is poisoned, so none of the
// two fields is initialized.
union Data<T, F> {
    value: ManuallyDrop<T>,
    f: ManuallyDrop<F>,
}

/// A value which is initialized on the first access.
pub struct LazyLock<T, F = fn() -> T> {
    // FIXME(nonpoison_once): if possible, switch to nonpoison version once it is available
    once: Once,
    data: UnsafeCell<Data<T, F>>,
}

impl<T, F: FnOnce() -> T> LazyLock<T, F> {
    /// Creates a new lazy value with the given initializing function.
    #[inline]
    pub const fn new(f: F) -> LazyLock<T, F> {
        LazyLock {
            once: Once::new(),
            data: UnsafeCell::new(Data {
                f: ManuallyDrop::new(f),
            }),
        }
    }

    /// Consumes this `LazyLock` returning the stored value.
    ///
    /// Returns `Ok(value)` if `Lazy` is initialized and `Err(f)` otherwise.
    pub fn into_inner(mut this: Self) -> Result<T, F> {
        let state = this.once.state();
        match state {
            OnceExclusiveState::Poisoned => panic_poisoned(),
            state => {
                let this = ManuallyDrop::new(this);
                let data = unsafe { ptr::read(&this.data) }.into_inner();
                match state {
                    OnceExclusiveState::Incomplete => {
                        Err(ManuallyDrop::into_inner(unsafe { data.f }))
                    }
                    OnceExclusiveState::Complete => {
                        Ok(ManuallyDrop::into_inner(unsafe { data.value }))
                    }
                    OnceExclusiveState::Poisoned => unreachable!(),
                }
            }
        }
    }

    /// Forces the evaluation of this lazy value and returns a mutable reference to
    /// the result.

    #[inline]
    pub fn force_mut(this: &mut LazyLock<T, F>) -> &mut T {
        #[cold]
        /// # Safety
        /// May only be called when the state is `Incomplete`.
        unsafe fn really_init_mut<T, F: FnOnce() -> T>(this: &mut LazyLock<T, F>) -> &mut T {
            struct PoisonOnPanic<'a, T, F>(&'a mut LazyLock<T, F>);
            impl<T, F> Drop for PoisonOnPanic<'_, T, F> {
                #[inline]
                fn drop(&mut self) {
                    self.0.once.set_state(OnceExclusiveState::Poisoned);
                }
            }

            // SAFETY: We always poison if the initializer panics (then we never check the data),
            // or set the data on success.
            let f = unsafe { ManuallyDrop::take(&mut this.data.get_mut().f) };
            // INVARIANT: Initiated from mutable reference, don't drop because we read it.
            let guard = PoisonOnPanic(this);
            let data = f();
            guard.0.data.get_mut().value = ManuallyDrop::new(data);
            guard.0.once.set_state(OnceExclusiveState::Complete);
            core::mem::forget(guard);
            // SAFETY: We put the value there above.
            unsafe { &mut this.data.get_mut().value }
        }

        let state = this.once.state();
        match state {
            OnceExclusiveState::Poisoned => panic_poisoned(),
            // SAFETY: The `Once` states we completed the initialization.
            OnceExclusiveState::Complete => unsafe { &mut this.data.get_mut().value },
            // SAFETY: The state is `Incomplete`.
            OnceExclusiveState::Incomplete => unsafe { really_init_mut(this) },
        }
    }

    /// Forces the evaluation of this lazy value and returns a reference to
    /// result. This is equivalent to the `Deref` impl, but is explicit.
    #[inline]
    pub fn force(this: &LazyLock<T, F>) -> &T {
        this.once.call_once_force(|state| {
            if state.is_poisoned() {
                panic_poisoned();
            }

            // SAFETY: `call_once` only runs this closure once, ever.
            let data = unsafe { &mut *this.data.get() };
            let f = unsafe { ManuallyDrop::take(&mut data.f) };
            let value = f();
            data.value = ManuallyDrop::new(value);
        });

        // SAFETY:
        // There are four possible scenarios:
        // * the closure was called and initialized `value`.
        // * the closure was called and panicked, so this point is never reached.
        // * the closure was not called, but a previous call initialized `value`.
        // * the closure was not called because the Once is poisoned, which we handled above.
        // So `value` has definitely been initialized and will not be modified again.
        unsafe { &*(*this.data.get()).value }
    }
}

impl<T, F> LazyLock<T, F> {
    /// Returns a mutable reference to the value if initialized. Otherwise (if uninitialized or
    /// poisoned), returns `None`.
    #[inline]
    pub fn get_mut(this: &mut LazyLock<T, F>) -> Option<&mut T> {
        // `state()` does not perform an atomic load, so prefer it over `is_complete()`.
        let state = this.once.state();
        match state {
            // SAFETY:
            // The closure has been run successfully, so `value` has been initialized.
            OnceExclusiveState::Complete => Some(unsafe { &mut this.data.get_mut().value }),
            _ => None,
        }
    }

    /// Returns a reference to the value if initialized. Otherwise (if uninitialized or poisoned),
    /// returns `None`.
    #[inline]
    pub fn get(this: &LazyLock<T, F>) -> Option<&T> {
        if this.once.is_completed() {
            // SAFETY:
            // The closure has been run successfully, so `value` has been initialized
            // and will not be modified again.
            Some(unsafe { &(*this.data.get()).value })
        } else {
            None
        }
    }
}

impl<T, F> Drop for LazyLock<T, F> {
    fn drop(&mut self) {
        match self.once.state() {
            OnceExclusiveState::Incomplete => unsafe {
                ManuallyDrop::drop(&mut self.data.get_mut().f)
            },
            OnceExclusiveState::Complete => unsafe {
                ManuallyDrop::drop(&mut self.data.get_mut().value)
            },
            OnceExclusiveState::Poisoned => {}
        }
    }
}

impl<T, F: FnOnce() -> T> Deref for LazyLock<T, F> {
    type Target = T;

    /// Dereferences the value.
    ///
    /// This method will block the calling thread if another initialization
    /// routine is currently running.
    ///
    /// # Panics
    ///
    /// If the initialization closure panics (the one that is passed to the [`new()`] method), the
    /// panic is propagated to the caller, and the lock becomes poisoned. This will cause all future
    /// accesses of the lock (via [`force()`] or a dereference) to panic.
    ///
    /// [`new()`]: LazyLock::new
    /// [`force()`]: LazyLock::force
    #[inline]
    fn deref(&self) -> &T {
        LazyLock::force(self)
    }
}

impl<T, F: FnOnce() -> T> DerefMut for LazyLock<T, F> {
    /// # Panics
    ///
    /// If the initialization closure panics (the one that is passed to the [`new()`] method), the
    /// panic is propagated to the caller, and the lock becomes poisoned. This will cause all future
    /// accesses of the lock (via [`force()`] or a dereference) to panic.
    ///
    /// [`new()`]: LazyLock::new
    /// [`force()`]: LazyLock::force
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        LazyLock::force_mut(self)
    }
}

impl<T: fmt::Debug, F> fmt::Debug for LazyLock<T, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut d = f.debug_tuple("LazyLock");
        match LazyLock::get(self) {
            Some(v) => d.field(v),
            None => d.field(&format_args!("<uninit>")),
        };
        d.finish()
    }
}

impl<T, F> From<T> for LazyLock<T, F> {
    /// Constructs a `LazyLock` that starts already initialized
    /// with the provided value.
    #[inline]
    fn from(value: T) -> Self {
        LazyLock {
            once: Once::new_complete(),
            data: UnsafeCell::new(Data {
                value: ManuallyDrop::new(value),
            }),
        }
    }
}

#[cold]
#[inline(never)]
fn panic_poisoned() -> ! {
    panic!("LazyLock instance has previously been poisoned")
}

// We never create a `&F` from a `&LazyLock<T, F>` so it is fine
// to not impl `Sync` for `F`.
unsafe impl<T: Sync + Send, F: Send> Sync for LazyLock<T, F> {}
// auto-derived `Send` impl is OK.
