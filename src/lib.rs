#![no_std]

use core::ops::Deref;

use spin::Mutex;
pub mod errors {
    pub use safa_utils::errors::{ErrorStatus, SysResult};
}

pub mod alloc;
pub mod raw;
pub mod syscalls;

pub struct Locked<T> {
    inner: Mutex<T>,
}

impl<T> Locked<T> {
    pub(crate) const fn new(inner: T) -> Self {
        Self {
            inner: Mutex::new(inner),
        }
    }
}

impl<T> Deref for Locked<T> {
    type Target = Mutex<T>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
