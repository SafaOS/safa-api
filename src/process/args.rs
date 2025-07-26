//! Wrapper around the arguments passed to the program.
//! api should be initialized before use see [`super::init`]

use safa_abi::ffi::{option::OptZero, str::Str};

use crate::exported_func;
use core::{cell::UnsafeCell, mem::MaybeUninit, ptr::NonNull};

// args

#[derive(Debug, Clone, Copy)]
pub(super) struct RawArgs {
    args: NonNull<[&'static str]>,
}

impl RawArgs {
    pub const fn new(args: Option<NonNull<[&'static str]>>) -> Self {
        Self {
            args: match args {
                Some(args) => args,
                None => unsafe { NonNull::new_unchecked(&mut []) },
            },
        }
    }

    const fn len(&self) -> usize {
        unsafe { self.args.as_ref().len() }
    }

    fn get(&self, index: usize) -> Option<&'static str> {
        unsafe { self.args.as_ref().get(index).copied() }
    }

    const unsafe fn into_slice(self) -> &'static [&'static str] {
        unsafe { self.args.as_ref() }
    }
}

pub(super) struct RawArgsStatic(UnsafeCell<MaybeUninit<RawArgs>>);
unsafe impl Sync for RawArgsStatic {}

impl RawArgsStatic {
    pub const fn new() -> Self {
        Self(UnsafeCell::new(MaybeUninit::uninit()))
    }

    pub unsafe fn init(&self, args: RawArgs) {
        unsafe {
            self.0.get().write(MaybeUninit::new(args));
        }
    }

    const unsafe fn get_unchecked(&self) -> &mut RawArgs {
        (*self.0.get()).assume_init_mut()
    }

    unsafe fn get(&self, index: usize) -> Option<&'static str> {
        unsafe { self.get_unchecked().get(index) }
    }

    const unsafe fn len(&self) -> usize {
        unsafe { self.get_unchecked().len() }
    }

    pub const unsafe fn as_slice(&self) -> &'static [&'static str] {
        unsafe {
            let raw = self.get_unchecked();
            raw.into_slice()
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
    pub extern "C" fn sysget_arg(index: usize) -> OptZero<Str> {
        unsafe { RAW_ARGS.get(index).map(|s| Str::from_str(s)).into() }
    }
}

/// An iterator over the arguments passed to the program.
pub struct ArgsIter {
    args: &'static [&'static str],
    index: usize,
}

impl ArgsIter {
    pub fn get() -> Self {
        let args = unsafe { RAW_ARGS.as_slice() };
        Self { args, index: 0 }
    }

    pub fn get_index(&self, index: usize) -> Option<&'static str> {
        self.args.get(index).copied()
    }

    pub fn next(&mut self) -> Option<&'static str> {
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
