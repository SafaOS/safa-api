pub(crate) trait IntoSyscallArg {
    fn into_syscall_arg(self) -> usize;
}

macro_rules! impl_into_syscall_as {
    ($ty:ty) => {
        impl IntoSyscallArg for $ty {
            #[inline(always)]
            fn into_syscall_arg(self) -> usize {
                self as usize
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

impl<T> IntoSyscallArg for *const T {
    #[inline(always)]
    fn into_syscall_arg(self) -> usize {
        self as usize
    }
}

impl<T> IntoSyscallArg for *mut T {
    #[inline(always)]
    fn into_syscall_arg(self) -> usize {
        self as usize
    }
}

// used by Documentation
#[allow(unused)]
use super::ErrorStatus;

/// A nullable muttable pointer to `T`
///
/// garuanteed to be accepted by the syscall if it is null,
/// however the syscall will return [`ErrorStatus::InvaildPtr`] if it is not aligned to `align_of::<T>()`
///
/// this is typically for the syscall to return optional values
pub type OptionalPtrMut<T> = *mut T;

/// A nullable imuttable pointer to `T`
///
/// garuanteed to be accepted by the syscall if it is null,
/// however the syscall will return [`ErrorStatus::InvaildPtr`] if it is not aligned to `align_of::<T>()`
///
/// this is typically for the syscall to return optional values
pub type OptionalPtr<T> = *const T;

/// A muttable pointer to `T`
///
/// the syscall will return [`ErrorStatus::InvaildPtr`] if it is not aligned to `align_of::<T>()` or if it is null
///
/// typically used for the syscall to return a value
pub type RequiredPtrMut<T> = *mut T;

/// An immuttable pointer to `T`
///
/// the syscall will return [`ErrorStatus::InvaildPtr`] if it is not aligned to `align_of::<T>()` or if it is null
///
/// typically used for the syscall to return a value
pub type RequiredPtr<T> = *const T;

/// An immuttable pointer to a byte array
///
/// the syscall will return [`ErrorStatus::InvaildPtr`] if it is null
///
/// the syscall will return [`ErrorStatus::InvaildStr`] if it is not valid utf-8
///
/// typically followed by a `len` parameter to specify the length of the string
pub type StrPtr = RequiredPtr<u8>;

/// A muttable pointer to a byte array
///
/// the syscall will return [`ErrorStatus::InvaildPtr`] if it is null
///
/// typically used for the syscall to return a string meaning that after the syscall is successful it should contain a valid utf-8 string
///
/// typically followed by a `len` parameter to specify the length of the string
pub type StrPtrMut = RequiredPtrMut<u8>;

/// An optional immuttable nullable pointer to a byte array
///
/// the syscall will return [`ErrorStatus::InvaildStr`] if it is not null and is not valid utf-8
///
/// typically followed by a `len` parameter to specify the length of the string
///
/// can be null
pub type OptionalStrPtr = OptionalPtr<u8>;
