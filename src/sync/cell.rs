use core::{
    cell::UnsafeCell,
    marker::PhantomData,
    ops::Deref,
    sync::atomic::{AtomicBool, Ordering},
};

use crate::syscalls;

enum LazyData<T, F: FnOnce() -> T> {
    Uninitialized(F),
    Initialized(T),
    Initializing,
}

impl<T, F: FnOnce() -> T> LazyData<T, F> {
    fn get_value(&self) -> Option<&T> {
        match self {
            LazyData::Initialized(ref value) => Some(value),
            _ => None,
        }
    }

    fn start_initialize(&mut self) -> F {
        match self {
            LazyData::Uninitialized(_) => {
                let LazyData::Uninitialized(res) = core::mem::replace(self, LazyData::Initializing)
                else {
                    unreachable!()
                };
                res
            }
            _ => panic!("LazyData::start_initialize called on initialized data"),
        }
    }
}

/// Synchronous Lazily initialized value
pub struct LazyCell<T> {
    running_init: AtomicBool,
    value: UnsafeCell<LazyData<T, fn() -> T>>,
    _marker: PhantomData<T>,
}

impl<T> LazyCell<T> {
    pub const fn new(call: fn() -> T) -> Self {
        Self {
            running_init: AtomicBool::new(false),
            value: UnsafeCell::new(LazyData::Uninitialized(call)),
            _marker: PhantomData,
        }
    }

    /// Gets the value or initializes it synchronously if not already initialized.
    pub fn get(&self) -> &T {
        let wait_for_init = || {
            while self.running_init.load(Ordering::Acquire) {
                syscalls::thread::yield_now();
            }

            unsafe {
                (&*self.value.get())
                    .get_value()
                    .expect("Lazy awaited initialization but the value was never initialized")
            }
        };

        match unsafe { &*self.value.get() } {
            LazyData::Initialized(ref value) => value,
            LazyData::Initializing => wait_for_init(),
            LazyData::Uninitialized(_) => {
                if self
                    .running_init
                    .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
                    .is_err()
                {
                    wait_for_init()
                } else {
                    let f = unsafe { (*self.value.get()).start_initialize() };
                    let value = (f)();
                    unsafe {
                        *self.value.get() = LazyData::Initialized(value);
                    }

                    self.running_init.store(false, Ordering::Release);
                    unsafe { (&*self.value.get()).get_value().unwrap() }
                }
            }
        }
    }
}

impl<T> Deref for LazyCell<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.get()
    }
}

unsafe impl<T: Send> Send for LazyCell<T> {}
unsafe impl<T: Sync> Sync for LazyCell<T> {}
