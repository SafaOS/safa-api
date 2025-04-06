#![no_std]
#![feature(naked_functions)]

use core::arch::asm;
use core::arch::naked_asm;
use core::mem::MaybeUninit;
use core::ptr::NonNull;

unsafe extern "C" {
    fn main(argc: i32, argv: *const NonNull<u8>) -> i32;
}

#[inline(always)]
unsafe fn exit(code: usize) -> ! {
    unsafe { asm!("xor rax, rax", "int 0x80", in("rdi") code, options(noreturn)) }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn _start_inner(argc: usize, argv: *mut (NonNull<u8>, usize)) -> ! {
    let argv_slice = unsafe { core::slice::from_raw_parts(argv, argc) };
    let bytes = argc * size_of::<usize>();

    alloca::with_alloca(bytes, |c_argv_slice: &mut [MaybeUninit<u8>]| {
        let c_argv_slice_ptr = c_argv_slice.as_mut_ptr() as *mut NonNull<u8>;
        let c_argv_slice = unsafe { core::slice::from_raw_parts_mut(c_argv_slice_ptr, argc) };

        for (i, (arg_ptr, _)) in argv_slice.iter().enumerate() {
            c_argv_slice[i] = *arg_ptr;
        }

        unsafe {
            let result = main(argc as i32, c_argv_slice_ptr as *const _);
            exit(result as usize)
        }
    })
}

#[unsafe(no_mangle)]
#[naked]
extern "C" fn _start() {
    unsafe {
        naked_asm!("xor rbp, rbp", "push rbp", "push rbp", "call _start_inner");
    }
}
