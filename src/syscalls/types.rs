// ==================== ARGS ========================
pub(crate) trait IntoSyscallArg {
    type RegResults;
    fn into_syscall_arg(self) -> Self::RegResults;
}

macro_rules! impl_into_syscall_as {
    ($ty:ty) => {
        impl IntoSyscallArg for $ty {
            type RegResults = (usize,);
            #[inline(always)]
            fn into_syscall_arg(self) -> (usize,) {
                (self as usize,)
            }
        }
    };
}

impl_into_syscall_as!(usize);
impl_into_syscall_as!(isize);
impl_into_syscall_as!(u8);
impl_into_syscall_as!(i8);
impl_into_syscall_as!(u16);
impl_into_syscall_as!(i16);
impl_into_syscall_as!(u32);
impl_into_syscall_as!(i32);
impl_into_syscall_as!(u64);
impl_into_syscall_as!(i64);
impl_into_syscall_as!(u128);
impl_into_syscall_as!(i128);
impl_into_syscall_as!(bool);

impl IntoSyscallArg for (usize,) {
    type RegResults = (usize,);

    #[inline(always)]
    fn into_syscall_arg(self) -> (usize,) {
        self
    }
}

impl<T> IntoSyscallArg for *const T {
    type RegResults = (usize,);

    #[inline(always)]
    fn into_syscall_arg(self) -> (usize,) {
        (self as usize,)
    }
}

impl<T> IntoSyscallArg for *mut T {
    type RegResults = (usize,);

    #[inline(always)]
    fn into_syscall_arg(self) -> (usize,) {
        (self as usize,)
    }
}

impl IntoSyscallArg for safa_abi::fs::OpenOptions {
    type RegResults = (usize,);

    #[inline(always)]
    fn into_syscall_arg(self) -> (usize,) {
        let bits: u8 = unsafe { core::mem::transmute(self) };
        bits.into_syscall_arg()
    }
}

impl<T> IntoSyscallArg for Slice<T> {
    type RegResults = (usize, usize);

    #[inline(always)]
    fn into_syscall_arg(self) -> (usize, usize) {
        (self.as_ptr() as usize, self.len())
    }
}

impl IntoSyscallArg for Str {
    type RegResults = (usize, usize);

    #[inline(always)]
    fn into_syscall_arg(self) -> (usize, usize) {
        (self.as_ptr() as usize, self.len())
    }
}

impl<T> IntoSyscallArg for FFINonNull<T> {
    type RegResults = (usize,);
    fn into_syscall_arg(self) -> Self::RegResults {
        (self.as_ptr() as usize,)
    }
}

impl<T: NotZeroable + IntoSyscallArg> IntoSyscallArg for OptZero<T> {
    type RegResults = T::RegResults;
    fn into_syscall_arg(self) -> Self::RegResults {
        let inner = unsafe { self.into_inner_unchecked() };
        inner.into_syscall_arg()
    }
}

impl IntoSyscallArg for SockDomain {
    type RegResults = (usize,);
    fn into_syscall_arg(self) -> Self::RegResults {
        let u8: u8 = unsafe { core::mem::transmute(self) };
        (u8 as usize,)
    }
}

// ==================== Return ========================

pub trait OkSyscallResult {
    fn from_usize(value: usize) -> Self;
}

impl OkSyscallResult for Infallible {
    fn from_usize(value: usize) -> Self {
        unreachable!("Syscall shouldn't return anything, returned: {value}")
    }
}

impl OkSyscallResult for () {
    fn from_usize(value: usize) -> Self {
        _ = value;
    }
}

impl OkSyscallResult for usize {
    fn from_usize(value: usize) -> Self {
        value
    }
}

impl OkSyscallResult for u32 {
    fn from_usize(value: usize) -> Self {
        value as u32
    }
}

impl<T> OkSyscallResult for NonNull<T> {
    fn from_usize(value: usize) -> Self {
        NonNull::new(value as *mut T).expect("Syscall shouldn't return a null pointer")
    }
}

use core::convert::Infallible;
use core::marker::PhantomData;
use core::ptr::NonNull;
use safa_abi::errors::SysResult;

/// An opaque type that represents a syscall result
/// the underlying type is a usize signed integer, in which 0..=isize::MAX is success and any negative value is in error
/// represented by the -[`ErrorStatus`] enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct SyscallResults<T: OkSyscallResult = ()> {
    inner: SysResult,
    _marker: PhantomData<T>,
}

impl<T: OkSyscallResult> SyscallResults<T> {
    pub fn get(self) -> Result<T, ErrorStatus> {
        self.inner.into_result().map(|ok| T::from_usize(ok))
    }
}

use safa_abi::ffi::option::OptZero;
use safa_abi::ffi::{ptr::FFINonNull, slice::Slice, str::Str};

use safa_abi::ffi::NotZeroable;
use safa_abi::sockets::SockDomain;

use crate::errors::ErrorStatus;

/// A nullable muttable pointer to `T`
///
/// guaranteed to be accepted by the syscall if it is null,
/// however the syscall will return [`ErrorStatus::InvalidPtr`] if it is not aligned to `align_of::<T>()`
///
/// this is typically for the syscall to return optional values
pub type OptionalPtrMut<T> = OptZero<RequiredPtrMut<T>>;

/// A nullable imuttable pointer to `T`
///
/// guaranteed to be accepted by the syscall if it is null,
/// however the syscall will return [`ErrorStatus::InvalidPtr`] if it is not aligned to `align_of::<T>()`
///
/// this is typically for the syscall to return optional values
pub type OptionalPtr<T> = OptZero<RequiredPtr<T>>;

/// A muttable pointer to `T`
///
/// the syscall will return [`ErrorStatus::InvalidPtr`] if it is not aligned to `align_of::<T>()` or if it is null
///
/// typically used for the syscall to return a value
pub type RequiredPtrMut<T> = FFINonNull<T>;

/// An immuttable pointer to `T`
///
/// the syscall will return [`ErrorStatus::InvalidPtr`] if it is not aligned to `align_of::<T>()` or if it is null
///
/// typically used for the syscall to return a value
pub type RequiredPtr<T> = FFINonNull<T>;

/// An optional immuttable nullable utf8 byte slice
///
/// the syscall will return [`ErrorStatus::InvalidStr`] if it is not null and is not valid utf-8
///
/// typically followed by a `len` parameter to specify the length of the string
///
/// can be null
pub type OptionalStr = OptZero<Str>;

/// An optional muttable nullable utf8 byte slice
///
/// the syscall will return [`ErrorStatus::InvalidStr`] if it is not null and is not valid utf-8
///
/// typically followed by a `len` parameter to specify the length of the string
///
/// can be null
pub type OptionalStrMut = OptZero<Str>;

/// A process ID
pub type Pid = u32;
/// A thread ID
pub type Tid = u32;

/// A resource id
/// this is a generic type that can be used to represent any resource (file, directory, device, directory iterator, etc.)
pub type Ri = u32;
