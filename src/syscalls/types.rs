pub trait IntoSyscallArg {
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
