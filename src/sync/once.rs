use crate::sync::futex::{futex_wait, futex_wake_all, Futex, Primitive};
use core::cell::Cell;
use core::convert::Infallible;
use core::sync::atomic::Ordering::{Acquire, Relaxed, Release};

// On some platforms, the OS is very nice and handles the waiter queue for us.
// This means we only need one atomic value with 4 states:

/// No initialization has run yet, and no thread is currently using the Once.
const INCOMPLETE: Primitive = 3;
/// Some thread has previously attempted to initialize the Once, but it panicked,
/// so the Once is now poisoned. There are no other threads currently accessing
/// this Once.
const POISONED: Primitive = 2;
/// Some thread is currently attempting to run initialization. It may succeed,
/// so all future threads need to wait for it to finish.
const RUNNING: Primitive = 1;
/// Initialization has completed and all future calls should finish immediately.
/// By choosing this state as the all-zero state the `is_completed` check can be
/// a bit faster on some platforms.
const COMPLETE: Primitive = 0;

// An additional bit indicates whether there are waiting threads:

/// May only be set if the state is not COMPLETE.
const QUEUED: Primitive = 4;

// Threads wait by setting the QUEUED bit and calling `futex_wait` on the state
// variable. When the running thread finishes, it will wake all waiting threads using
// `futex_wake_all`.

const STATE_MASK: Primitive = 0b11;

pub struct OnceState {
    poisoned: bool,
    set_state_to: Cell<Primitive>,
}

impl OnceState {
    #[inline]
    pub fn is_poisoned(&self) -> bool {
        self.poisoned
    }

    #[inline]
    pub fn poison(&self) {
        self.set_state_to.set(POISONED);
    }
}

struct CompletionGuard<'a> {
    state_and_queued: &'a Futex,
    set_state_on_drop_to: Primitive,
}

impl<'a> Drop for CompletionGuard<'a> {
    fn drop(&mut self) {
        // Use release ordering to propagate changes to all threads checking
        // up on the Once. `futex_wake_all` does its own synchronization, hence
        // we do not need `AcqRel`.
        if self
            .state_and_queued
            .swap(self.set_state_on_drop_to, Release)
            & QUEUED
            != 0
        {
            futex_wake_all(self.state_and_queued);
        }
    }
}

pub struct OnceInner {
    state_and_queued: Futex,
}

impl OnceInner {
    #[inline]
    pub const fn new() -> OnceInner {
        OnceInner {
            state_and_queued: Futex::new(INCOMPLETE),
        }
    }

    #[inline]
    pub const fn new_complete() -> OnceInner {
        OnceInner {
            state_and_queued: Futex::new(COMPLETE),
        }
    }

    #[inline]
    pub fn is_completed(&self) -> bool {
        // Use acquire ordering to make all initialization changes visible to the
        // current thread.
        self.state_and_queued.load(Acquire) == COMPLETE
    }

    #[inline]
    pub(crate) fn state(&mut self) -> OnceExclusiveState {
        match *self.state_and_queued.get_mut() {
            INCOMPLETE => OnceExclusiveState::Incomplete,
            POISONED => OnceExclusiveState::Poisoned,
            COMPLETE => OnceExclusiveState::Complete,
            _ => unreachable!("invalid Once state"),
        }
    }

    #[inline]
    pub(crate) fn set_state(&mut self, new_state: OnceExclusiveState) {
        *self.state_and_queued.get_mut() = match new_state {
            OnceExclusiveState::Incomplete => INCOMPLETE,
            OnceExclusiveState::Poisoned => POISONED,
            OnceExclusiveState::Complete => COMPLETE,
        };
    }

    #[cold]
    #[track_caller]
    pub fn wait(&self, ignore_poisoning: bool) {
        let mut state_and_queued = self.state_and_queued.load(Acquire);
        loop {
            let state = state_and_queued & STATE_MASK;
            let queued = state_and_queued & QUEUED != 0;
            match state {
                COMPLETE => return,
                POISONED if !ignore_poisoning => {
                    // Panic to propagate the poison.
                    panic!("Once instance has previously been poisoned");
                }
                _ => {
                    // Set the QUEUED bit if it has not already been set.
                    if !queued {
                        state_and_queued += QUEUED;
                        if let Err(new) = self.state_and_queued.compare_exchange_weak(
                            state,
                            state_and_queued,
                            Relaxed,
                            Acquire,
                        ) {
                            state_and_queued = new;
                            continue;
                        }
                    }

                    futex_wait(&self.state_and_queued, state_and_queued, None);
                    state_and_queued = self.state_and_queued.load(Acquire);
                }
            }
        }
    }

    #[cold]
    #[track_caller]
    pub fn call(&self, ignore_poisoning: bool, f: &mut dyn FnMut(&OnceState)) {
        let mut state_and_queued = self.state_and_queued.load(Acquire);
        loop {
            let state = state_and_queued & STATE_MASK;
            let queued = state_and_queued & QUEUED != 0;
            match state {
                COMPLETE => return,
                POISONED if !ignore_poisoning => {
                    // Panic to propagate the poison.
                    panic!("Once instance has previously been poisoned");
                }
                INCOMPLETE | POISONED => {
                    // Try to register the current thread as the one running.
                    let next = RUNNING + if queued { QUEUED } else { 0 };
                    if let Err(new) = self.state_and_queued.compare_exchange_weak(
                        state_and_queued,
                        next,
                        Acquire,
                        Acquire,
                    ) {
                        state_and_queued = new;
                        continue;
                    }

                    // `waiter_queue` will manage other waiting threads, and
                    // wake them up on drop.
                    let mut waiter_queue = CompletionGuard {
                        state_and_queued: &self.state_and_queued,
                        set_state_on_drop_to: POISONED,
                    };
                    // Run the function, letting it know if we're poisoned or not.
                    let f_state = OnceState {
                        poisoned: state == POISONED,
                        set_state_to: Cell::new(COMPLETE),
                    };
                    f(&f_state);
                    waiter_queue.set_state_on_drop_to = f_state.set_state_to.get();
                    return;
                }
                _ => {
                    // All other values must be RUNNING.
                    assert!(state == RUNNING);

                    // Set the QUEUED bit if it is not already set.
                    if !queued {
                        state_and_queued += QUEUED;
                        if let Err(new) = self.state_and_queued.compare_exchange_weak(
                            state,
                            state_and_queued,
                            Relaxed,
                            Acquire,
                        ) {
                            state_and_queued = new;
                            continue;
                        }
                    }

                    futex_wait(&self.state_and_queued, state_and_queued, None);
                    state_and_queued = self.state_and_queued.load(Acquire);
                }
            }
        }
    }
}

use core::fmt;

/// A low-level synchronization primitive for one-time global execution.
pub struct Once {
    inner: OnceInner,
}

pub(crate) enum OnceExclusiveState {
    Incomplete,
    Poisoned,
    Complete,
}

impl Once {
    /// Creates a new `Once` value.
    #[inline]
    #[must_use]
    pub const fn new() -> Once {
        Once {
            inner: OnceInner::new(),
        }
    }

    /// Creates a new `Once` value that starts already completed.
    #[inline]
    #[must_use]
    pub(crate) const fn new_complete() -> Once {
        Once {
            inner: OnceInner::new_complete(),
        }
    }

    /// Performs an initialization routine once and only once. The given closure
    /// will be executed if this is the first time `call_once` has been called,
    /// and otherwise the routine will *not* be invoked.
    #[inline]
    #[track_caller]
    pub fn call_once<F>(&self, f: F)
    where
        F: FnOnce(),
    {
        // Fast path check
        if self.inner.is_completed() {
            return;
        }

        let mut f = Some(f);
        self.inner.call(false, &mut |_| f.take().unwrap()());
    }

    /// Performs the same function as [`call_once()`] except ignores poisoning.
    #[inline]
    pub fn call_once_force<F>(&self, f: F)
    where
        F: FnOnce(&OnceState),
    {
        // Fast path check
        if self.inner.is_completed() {
            return;
        }

        let mut f = Some(f);
        self.inner.call(true, &mut |p| f.take().unwrap()(p));
    }

    /// Returns `true` if some [`call_once()`] call has completed
    /// successfully. Specifically, `is_completed` will return false in
    /// the following situations:
    ///   * [`call_once()`] was not called at all,
    ///   * [`call_once()`] was called, but has not yet completed,
    ///   * the [`Once`] instance is poisoned
    #[inline]
    pub fn is_completed(&self) -> bool {
        self.inner.is_completed()
    }

    /// Blocks the current thread until initialization has completed.
    pub fn wait(&self) {
        if !self.inner.is_completed() {
            self.inner.wait(false);
        }
    }

    /// Blocks the current thread until initialization has completed, ignoring
    /// poisoning.
    ///
    /// If this [`Once`] has been poisoned, this function blocks until it
    /// becomes completed, unlike [`Once::wait()`], which panics in this case.
    pub fn wait_force(&self) {
        if !self.inner.is_completed() {
            self.inner.wait(true);
        }
    }

    /// Returns the current state of the `Once` instance.
    ///
    /// Since this takes a mutable reference, no initialization can currently
    /// be running, so the state must be either "incomplete", "poisoned" or
    /// "complete".
    #[inline]
    pub(crate) fn state(&mut self) -> OnceExclusiveState {
        self.inner.state()
    }

    /// Sets current state of the `Once` instance.
    ///
    /// Since this takes a mutable reference, no initialization can currently
    /// be running, so the state must be either "incomplete", "poisoned" or
    /// "complete".
    #[inline]
    pub(crate) fn set_state(&mut self, new_state: OnceExclusiveState) {
        self.inner.set_state(new_state);
    }
}

impl fmt::Debug for Once {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Once").finish_non_exhaustive()
    }
}

impl fmt::Debug for OnceState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OnceState")
            .field("poisoned", &self.is_poisoned())
            .finish()
    }
}

use core::cell::UnsafeCell;
use core::marker::PhantomData;
use core::mem::MaybeUninit;

/// A synchronization primitive which can nominally be written to only once.
///
/// This type is a thread-safe [`OnceCell`], and can be used in statics.
/// In many simple cases, you can use [`LazyLock<T, F>`] instead to get the benefits of this type
/// with less effort: `LazyLock<T, F>` "looks like" `&T` because it initializes with `F` on deref!
/// Where OnceLock shines is when LazyLock is too simple to support a given case, as LazyLock
/// doesn't allow additional inputs to its function after you call [`LazyLock::new(|| ...)`].
///
/// A `OnceLock` can be thought of as a safe abstraction over uninitialized data that becomes
/// initialized once written.
pub struct OnceLock<T> {
    // FIXME(nonpoison_once): switch to nonpoison version once it is available
    once: Once,
    // Whether or not the value is initialized is tracked by `once.is_completed()`.
    value: UnsafeCell<MaybeUninit<T>>,
    /// `PhantomData` to make sure dropck understands we're dropping T in our Drop impl.
    ///
    /// ```compile_fail,E0597
    /// use std::sync::OnceLock;
    ///
    /// struct A<'a>(&'a str);
    ///
    /// impl<'a> Drop for A<'a> {
    ///     fn drop(&mut self) {}
    /// }
    ///
    /// let cell = OnceLock::new();
    /// {
    ///     let s = String::new();
    ///     let _ = cell.set(A(&s));
    /// }
    /// ```
    _marker: PhantomData<T>,
}

impl<T> OnceLock<T> {
    /// Creates a new uninitialized cell.
    #[inline]
    #[must_use]
    pub const fn new() -> OnceLock<T> {
        OnceLock {
            once: Once::new(),
            value: UnsafeCell::new(MaybeUninit::uninit()),
            _marker: PhantomData,
        }
    }

    /// Gets the reference to the underlying value.
    #[inline]
    pub fn get(&self) -> Option<&T> {
        if self.initialized() {
            // Safe b/c checked initialized
            Some(unsafe { self.get_unchecked() })
        } else {
            None
        }
    }

    /// Gets the mutable reference to the underlying value.
    #[inline]
    pub fn get_mut(&mut self) -> Option<&mut T> {
        if self.initialized_mut() {
            // Safe b/c checked initialized and we have a unique access
            Some(unsafe { self.get_unchecked_mut() })
        } else {
            None
        }
    }

    /// Blocks the current thread until the cell is initialized.
    #[inline]
    pub fn wait(&self) -> &T {
        self.once.wait_force();

        unsafe { self.get_unchecked() }
    }

    /// Initializes the contents of the cell to `value`.

    #[inline]
    pub fn set(&self, value: T) -> Result<(), T> {
        match self.try_insert(value) {
            Ok(_) => Ok(()),
            Err((_, value)) => Err(value),
        }
    }

    /// Initializes the contents of the cell to `value` if the cell was uninitialized,
    /// then returns a reference to it.
    ///
    /// May block if another thread is currently attempting to initialize the cell. The cell is
    /// guaranteed to contain a value when `try_insert` returns, though not necessarily the
    /// one provided.
    ///
    /// Returns `Ok(&value)` if the cell was uninitialized and
    /// `Err((&current_value, value))` if it was already initialized.
    #[inline]
    pub fn try_insert(&self, value: T) -> Result<&T, (&T, T)> {
        let mut value = Some(value);
        let res = self.get_or_init(|| value.take().unwrap());
        match value {
            None => Ok(res),
            Some(value) => Err((res, value)),
        }
    }

    /// Gets the contents of the cell, initializing it to `f()` if the cell
    /// was uninitialized.
    ///
    /// Many threads may call `get_or_init` concurrently with different
    /// initializing functions, but it is guaranteed that only one function
    /// will be executed if the function doesn't panic.
    ///
    /// # Panics
    ///
    /// If `f()` panics, the panic is propagated to the caller, and the cell
    /// remains uninitialized.
    ///
    /// It is an error to reentrantly initialize the cell from `f`. The
    /// exact outcome is unspecified. Current implementation deadlocks, but
    /// this may be changed to a panic in the future.
    #[inline]
    pub fn get_or_init<F>(&self, f: F) -> &T
    where
        F: FnOnce() -> T,
    {
        match self.get_or_try_init(|| Ok::<T, Infallible>(f())) {
            Ok(val) => val,
        }
    }

    /// Gets the mutable reference of the contents of the cell, initializing
    /// it to `f()` if the cell was uninitialized.
    #[inline]
    pub fn get_mut_or_init<F>(&mut self, f: F) -> &mut T
    where
        F: FnOnce() -> T,
    {
        match self.get_mut_or_try_init(|| Ok::<T, Infallible>(f())) {
            Ok(val) => val,
        }
    }

    /// Gets the contents of the cell, initializing it to `f()` if
    /// the cell was uninitialized. If the cell was uninitialized
    /// and `f()` failed, an error is returned.
    #[inline]
    pub fn get_or_try_init<F, E>(&self, f: F) -> Result<&T, E>
    where
        F: FnOnce() -> Result<T, E>,
    {
        // Fast path check
        // NOTE: We need to perform an acquire on the state in this method
        // in order to correctly synchronize `LazyLock::force`. This is
        // currently done by calling `self.get()`, which in turn calls
        // `self.initialized()`, which in turn performs the acquire.
        if let Some(value) = self.get() {
            return Ok(value);
        }
        self.initialize(f)?;

        // SAFETY: The inner value has been initialized
        Ok(unsafe { self.get_unchecked() })
    }

    /// Gets the mutable reference of the contents of the cell, initializing
    /// it to `f()` if the cell was uninitialized. If the cell was uninitialized
    /// and `f()` failed, an error is returned.
    #[inline]
    pub fn get_mut_or_try_init<F, E>(&mut self, f: F) -> Result<&mut T, E>
    where
        F: FnOnce() -> Result<T, E>,
    {
        if self.get_mut().is_none() {
            self.initialize(f)?;
        }

        // SAFETY: The inner value has been initialized
        Ok(unsafe { self.get_unchecked_mut() })
    }

    /// Consumes the `OnceLock`, returning the wrapped value. Returns
    /// `None` if the cell was uninitialized.
    #[inline]
    pub fn into_inner(mut self) -> Option<T> {
        self.take()
    }

    /// Takes the value out of this `OnceLock`, moving it back to an uninitialized state.
    #[inline]
    pub fn take(&mut self) -> Option<T> {
        if self.initialized_mut() {
            self.once = Once::new();
            // SAFETY: `self.value` is initialized and contains a valid `T`.
            // `self.once` is reset, so `initialized()` will be false again
            // which prevents the value from being read twice.
            unsafe { Some(self.value.get_mut().assume_init_read()) }
        } else {
            None
        }
    }

    #[inline]
    fn initialized(&self) -> bool {
        self.once.is_completed()
    }

    #[inline]
    fn initialized_mut(&mut self) -> bool {
        // `state()` does not perform an atomic load, so prefer it over `is_complete()`.
        let state = self.once.state();
        match state {
            OnceExclusiveState::Complete => true,
            _ => false,
        }
    }

    #[cold]
    fn initialize<F, E>(&self, f: F) -> Result<(), E>
    where
        F: FnOnce() -> Result<T, E>,
    {
        let mut res: Result<(), E> = Ok(());
        let slot = &self.value;

        // Ignore poisoning from other threads
        // If another thread panics, then we'll be able to run our closure
        self.once.call_once_force(|p| {
            match f() {
                Ok(value) => {
                    unsafe { (&mut *slot.get()).write(value) };
                }
                Err(e) => {
                    res = Err(e);

                    // Treat the underlying `Once` as poisoned since we
                    // failed to initialize our value.
                    p.poison();
                }
            }
        });
        res
    }

    /// # Safety
    ///
    /// The cell must be initialized
    #[inline]
    unsafe fn get_unchecked(&self) -> &T {
        debug_assert!(self.initialized());
        unsafe { (&*self.value.get()).assume_init_ref() }
    }

    /// # Safety
    ///
    /// The cell must be initialized
    #[inline]
    unsafe fn get_unchecked_mut(&mut self) -> &mut T {
        debug_assert!(self.initialized_mut());
        unsafe { self.value.get_mut().assume_init_mut() }
    }
}

// Why do we need `T: Send`?
// Thread A creates a `OnceLock` and shares it with
// scoped thread B, which fills the cell, which is
// then destroyed by A. That is, destructor observes
// a sent value.
unsafe impl<T: Sync + Send> Sync for OnceLock<T> {}
unsafe impl<T: Send> Send for OnceLock<T> {}

impl<T: fmt::Debug> fmt::Debug for OnceLock<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut d = f.debug_tuple("OnceLock");
        match self.get() {
            Some(v) => d.field(v),
            None => d.field(&format_args!("<uninit>")),
        };
        d.finish()
    }
}

impl<T: Clone> Clone for OnceLock<T> {
    #[inline]
    fn clone(&self) -> OnceLock<T> {
        let cell = Self::new();
        if let Some(value) = self.get() {
            match cell.set(value.clone()) {
                Ok(()) => (),
                Err(_) => unreachable!(),
            }
        }
        cell
    }
}

impl<T> From<T> for OnceLock<T> {
    /// Creates a new cell with its contents set to `value`.
    #[inline]
    fn from(value: T) -> Self {
        let cell = Self::new();
        match cell.set(value) {
            Ok(()) => cell,
            Err(_) => unreachable!(),
        }
    }
}

impl<T: PartialEq> PartialEq for OnceLock<T> {
    /// Equality for two `OnceLock`s.
    ///
    /// Two `OnceLock`s are equal if they either both contain values and their
    /// values are equal, or if neither contains a value.
    #[inline]
    fn eq(&self, other: &OnceLock<T>) -> bool {
        self.get() == other.get()
    }
}

impl<T: Eq> Eq for OnceLock<T> {}

impl<T> Drop for OnceLock<T> {
    #[inline]
    fn drop(&mut self) {
        if self.initialized_mut() {
            // SAFETY: The cell is initialized and being dropped, so it can't
            // be accessed again. We also don't touch the `T` other than
            // dropping it, which validates our usage of #[may_dangle].
            unsafe { self.value.get_mut().assume_init_drop() };
        }
    }
}
