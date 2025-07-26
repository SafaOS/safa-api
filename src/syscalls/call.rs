use crate::syscalls::types::SyscallResult;

use core::arch::asm;

/// Invokes a syscall with the given number and arguments
/// Number must be of type [`SyscallNum`]
/// Arguments must be of type [`usize`]
/// returns a [`SyscallResult`]
#[macro_export]
macro_rules! syscall {
    ($num: expr, $($arg: expr),*) => {{
        #[allow(unused_imports)]
        use $crate::syscalls::call::JoinTuples;
        use $crate::syscalls::call::SyscallCaller;
        #[allow(unused_imports)]
        use $crate::syscalls::types::IntoSyscallArg;

        let args = ();
        $(
            let args = args.join_tuple($arg.into_syscall_arg());
        )*
        SyscallCaller::<{ $num as u16 }, _>::new(args).call()
    }};
}

pub(crate) use syscall;

pub struct SyscallCaller<const NUM: u16, T> {
    args: T,
}

impl<const NUM: u16, T> SyscallCaller<NUM, T> {
    pub const fn new(args: T) -> Self {
        Self { args }
    }
}

impl<const NUM: u16> SyscallCaller<NUM, ()> {
    #[inline(always)]
    pub fn call(self) -> SyscallResult {
        syscall0::<NUM>()
    }
}

impl<const NUM: u16> SyscallCaller<NUM, (usize,)> {
    #[inline(always)]
    pub fn call(self) -> SyscallResult {
        syscall1::<NUM>(self.args.0)
    }
}

impl<const NUM: u16> SyscallCaller<NUM, (usize, usize)> {
    #[inline(always)]
    pub fn call(self) -> SyscallResult {
        syscall2::<NUM>(self.args.0, self.args.1)
    }
}

impl<const NUM: u16> SyscallCaller<NUM, (usize, usize, usize)> {
    #[inline(always)]
    pub fn call(self) -> SyscallResult {
        syscall3::<NUM>(self.args.0, self.args.1, self.args.2)
    }
}

impl<const NUM: u16> SyscallCaller<NUM, (usize, usize, usize, usize)> {
    #[inline(always)]
    pub fn call(self) -> SyscallResult {
        syscall4::<NUM>(self.args.0, self.args.1, self.args.2, self.args.3)
    }
}

impl<const NUM: u16> SyscallCaller<NUM, (usize, usize, usize, usize, usize)> {
    #[inline(always)]
    pub fn call(self) -> SyscallResult {
        syscall5::<NUM>(
            self.args.0,
            self.args.1,
            self.args.2,
            self.args.3,
            self.args.4,
        )
    }
}

#[doc(hidden)]
#[inline(always)]
pub fn syscall0<const NUM: u16>() -> SyscallResult {
    let result: u16;
    unsafe {
        #[cfg(target_arch = "x86_64")]
        asm!(
            "int 0x80",
            in("rax") NUM as usize,
            lateout("rax") result,
        );
        #[cfg(target_arch = "aarch64")]
        asm!(
            "svc #{num}",
            num = const NUM,
            lateout("x0") result
        );
        core::mem::transmute(result)
    }
}

#[doc(hidden)]
#[inline(always)]
pub fn syscall1<const NUM: u16>(arg1: usize) -> SyscallResult {
    let result: u16;
    unsafe {
        #[cfg(target_arch = "x86_64")]
        asm!(
            "int 0x80",
            in("rax") NUM as usize,
            in("rdi") arg1,
            lateout("rax") result,
        );
        #[cfg(target_arch = "aarch64")]
        asm!(
            "svc #{num}",
            num = const NUM,
            in("x0") arg1,
            lateout("x0") result
        );
        core::mem::transmute(result)
    }
}

#[doc(hidden)]
#[inline(always)]
pub fn syscall2<const NUM: u16>(arg1: usize, arg2: usize) -> SyscallResult {
    let result: u16;
    unsafe {
        #[cfg(target_arch = "x86_64")]
        asm!(
            "int 0x80",
            in("rax") NUM as usize,
            in("rdi") arg1,
            in("rsi") arg2,
            lateout("rax") result,
        );
        #[cfg(target_arch = "aarch64")]
        asm!(
            "svc #{num}",
            num = const NUM,
            in("x0") arg1,
            in("x1") arg2,
            lateout("x0") result
        );
        core::mem::transmute(result)
    }
}

#[doc(hidden)]
#[inline(always)]
pub fn syscall3<const NUM: u16>(arg1: usize, arg2: usize, arg3: usize) -> SyscallResult {
    let result: u16;
    unsafe {
        #[cfg(target_arch = "x86_64")]
        asm!(
            "int 0x80",
            in("rax") NUM as usize,
            in("rdi") arg1,
            in("rsi") arg2,
            in("rdx") arg3,
            lateout("rax") result,
        );
        #[cfg(target_arch = "aarch64")]
        asm!(
            "svc #{num}",
            num = const NUM,
            in("x0") arg1,
            in("x1") arg2,
            in("x2") arg3,
            lateout("x0") result
        );
        core::mem::transmute(result)
    }
}

#[doc(hidden)]
#[inline(always)]
pub fn syscall4<const NUM: u16>(
    arg1: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
) -> SyscallResult {
    let result: u16;
    unsafe {
        #[cfg(target_arch = "x86_64")]
        asm!(
            "int 0x80",
            in("rax") NUM as usize,
            in("rdi") arg1,
            in("rsi") arg2,
            in("rdx") arg3,
            in("rcx") arg4,
            lateout("rax") result,
        );

        #[cfg(target_arch = "aarch64")]
        asm!(
            "svc #{num}",
            num = const NUM,
            in("x0") arg1,
            in("x1") arg2,
            in("x2") arg3,
            in("x3") arg4,
            lateout("x0") result
        );
        core::mem::transmute(result)
    }
}

#[doc(hidden)]
#[inline(always)]
pub fn syscall5<const NUM: u16>(
    arg1: usize,
    arg2: usize,
    arg3: usize,
    arg4: usize,
    arg5: usize,
) -> SyscallResult {
    let result: u16;
    unsafe {
        #[cfg(target_arch = "x86_64")]
        asm!(
            "int 0x80",
            in("rax") NUM as usize,
            in("rdi") arg1,
            in("rsi") arg2,
            in("rdx") arg3,
            in("rcx") arg4,
            in("r8") arg5,
            lateout("rax") result,
        );
        #[cfg(target_arch = "aarch64")]
        asm!(
            "svc #{num}",
            num = const NUM,
            in("x0") arg1,
            in("x1") arg2,
            in("x2") arg3,
            in("x3") arg4,
            in("x4") arg5,
            lateout("x0") result
        );
        core::mem::transmute(result)
    }
}

pub trait JoinTuples<JoinWith> {
    type Output;
    fn join_tuple(self, other: JoinWith) -> Self::Output;
}

macro_rules! impl_join_single {
    ($($T:ident)*,$($O:ident)*) => {
        impl<$($T,)* $($O,)*> JoinTuples<($($O,)*)> for ($($T,)*) {
            type Output = ($($T,)* $($O,)*);
            fn join_tuple(self, other: ($($O,)*) ) -> Self::Output {
                #[allow(non_snake_case)]
                let ($($T,)*) = self;
                #[allow(non_snake_case)]
                let ($($O,)*) = other;
                ($($T,)* $($O,)*)
            }
        }
    };
}
macro_rules! impl_join {
    ($($T:ident)*,$($O:ident)*) => {
        impl_join_single!($($T)*,$($O)*);
        impl_join_single!($($O)*,$($T)*);
    };
}

macro_rules! impl_join_nothing {
    ($($T:ident)*) => {

        impl<$($T,)*> JoinTuples<()> for ($($T,)*) {
            type Output = ($($T,)*);
            fn join_tuple(self, _other: ()) -> Self::Output {
                self
            }
        }

        impl<$($T,)*> JoinTuples<($($T,)*)> for () {
            type Output = ($($T,)*);
            fn join_tuple(self, other: ($($T,)*)) -> Self::Output {
                _ = self;
                other
            }
        }
    };
}

// we only need to join up to 5 elements
impl_join_nothing!(A);
impl_join_single!(A, B);
impl_join_nothing!(A B);
impl_join_single!(A B, C D);
// joining A, B, C
impl_join!(A B, C);
impl_join_nothing!(A B C);
// ok we got A, B, C, joining A, B, C, D
impl_join!(A B C, D);
impl_join_nothing!(A B C D);
// we got A, B, C, D, joining A, B, C, D, E == 5
impl_join!(A B C D, E);
impl_join_nothing!(A B C D E);
// now joining A, B with C, D, E to get A, B, C, D, E == 5
impl_join!(A B C, D E);
