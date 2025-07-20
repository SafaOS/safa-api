//! Wrapper around the arguments passed to the program.
//! api should be initialized before use see [`super::init`]

use core::{cell::UnsafeCell, mem::MaybeUninit, ptr::NonNull};
use safa_abi::raw::{NonNullSlice, Optional};

use crate::exported_func;

// args

#[derive(Debug, Clone, Copy)]
pub(super) struct RawArgs {
    args: NonNull<[NonNullSlice<u8>]>,
}

impl RawArgs {
    pub const fn new(args: NonNull<[NonNullSlice<u8>]>) -> Self {
        Self { args }
    }

    fn len(&self) -> usize {
        unsafe { self.args.as_ref().len() }
    }

    fn get(&self, index: usize) -> Option<NonNullSlice<u8>> {
        unsafe { self.args.as_ref().get(index).copied() }
    }

    unsafe fn into_slice(self) -> &'static [NonNullSlice<u8>] {
        unsafe { self.args.as_ref() }
    }
}

pub(super) struct RawArgsStatic(UnsafeCell<MaybeUninit<Option<RawArgs>>>);
unsafe impl Sync for RawArgsStatic {}

impl RawArgsStatic {
    pub const fn new() -> Self {
        Self(UnsafeCell::new(MaybeUninit::uninit()))
    }

    pub unsafe fn init(&self, args: Option<RawArgs>) {
        unsafe {
            self.0.get().write(MaybeUninit::new(args));
        }
    }

    unsafe fn get(&self, index: usize) -> Option<NonNullSlice<u8>> {
        unsafe { (*self.0.get()).assume_init()?.get(index) }
    }

    unsafe fn len(&self) -> usize {
        if let Some(args) = unsafe { (*self.0.get()).assume_init() } {
            args.len()
        } else {
            0
        }
    }

    unsafe fn get_raw(&self) -> Option<RawArgs> {
        unsafe { (*self.0.get()).assume_init() }
    }

    pub unsafe fn as_slice(&self) -> &'static [NonNullSlice<u8>] {
        unsafe {
            if let Some(raw) = self.get_raw() {
                raw.into_slice()
            } else {
                &mut []
            }
        }
    }
}

pub(super) static RAW_ARGS: RawArgsStatic = RawArgsStatic::new();

exported_func! {
    /// Get the number of arguments passed to the program.
    pub extern "C" fn sysget_argc() -> usize {
        unsafe { RAW_ARGS.len() }
    }
}

exported_func! {
    /// Get the argument at the given index.
    pub extern "C" fn sysget_arg(index: usize) -> Optional<NonNullSlice<u8>> {
        unsafe { RAW_ARGS.get(index).into() }
    }
}

/// An iterator over the arguments passed to the program.
pub struct ArgsIter {
    args: &'static [NonNullSlice<u8>],
    index: usize,
}

impl ArgsIter {
    pub fn get() -> Self {
        let args = unsafe { RAW_ARGS.as_slice() };
        Self { args, index: 0 }
    }

    pub fn get_index(&self, index: usize) -> Option<NonNullSlice<u8>> {
        self.args.get(index).copied()
    }

    pub fn next(&mut self) -> Option<NonNullSlice<u8>> {
        if self.index < self.args.len() {
            let arg = self.args[self.index];
            self.index += 1;
            Some(arg)
        } else {
            None
        }
    }
    /// The total amount of args in the iterator before calling [`Self::next`]
    pub fn total_len(&self) -> usize {
        self.args.len()
    }
    /// The amount of remaining args in the iterator
    pub fn len(&self) -> usize {
        self.total_len() - self.index
    }
}
