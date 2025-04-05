pub use safa_abi::raw::*;

use crate::syscalls::types::IntoSyscallArg;

impl IntoSyscallArg for processes::SpawnFlags {
    #[inline(always)]
    fn into_syscall_arg(self) -> usize {
        let u8: u8 = unsafe { core::mem::transmute(self) };
        u8 as usize
    }
}
