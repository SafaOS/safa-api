//! contains api initialization functions, that should be called before using the api
use core::ptr::NonNull;

use crate::{
    alloc::GLOBAL_SYSTEM_ALLOCATOR,
    syscalls::{self},
};
use safa_abi::raw::{processes::AbiStructures, NonNullSlice, RawSliceMut};

use super::{
    args::{RawArgs, RAW_ARGS},
    env::RAW_ENV,
    stdio::init_meta,
};

// Initialization

fn init_args(args: RawSliceMut<NonNullSlice<u8>>) {
    unsafe {
        let slice = args
            .into_slice_mut()
            .map(|inner| RawArgs::new(NonNull::new_unchecked(inner as *mut _)));
        RAW_ARGS.init(slice)
    }
}

fn init_env(env: RawSliceMut<NonNullSlice<u8>>) {
    unsafe {
        let slice = env
            .into_slice_mut()
            .map(|inner| RawArgs::new(NonNull::new_unchecked(inner as *mut _)));
        RAW_ENV.init(slice)
    }
}

/// Initializes the safa-api
/// if your programs are designed as C main function,
///
/// use [`_c_api_init`] instead
#[cfg_attr(
    not(any(feature = "std", feature = "rustc-dep-of-std")),
    unsafe(no_mangle)
)]
#[inline(always)]
pub extern "C" fn sysapi_init(
    args: RawSliceMut<NonNullSlice<u8>>,
    env: RawSliceMut<NonNullSlice<u8>>,
    task_abi_structures: AbiStructures,
) {
    init_args(args);
    init_env(env);
    init_meta(task_abi_structures);
}

/// Initializes the safa-api, converts arguments to C-style arguments, calls `main`, and exits with the result
/// main are designed as C main function,
///
/// this function is designed to be called from C code at _start before main,
/// main should be passed as a parameter
#[cfg_attr(
    not(any(feature = "std", feature = "rustc-dep-of-std")),
    unsafe(no_mangle)
)]
pub unsafe extern "C" fn _c_api_init(
    args: RawSliceMut<NonNullSlice<u8>>,
    env: RawSliceMut<NonNullSlice<u8>>,
    task_abi_structures: *const AbiStructures,
    main: extern "C" fn(argc: i32, argv: *const *const u8) -> i32,
) -> ! {
    sysapi_init(args, env, *task_abi_structures);

    // Convert SafaOS `_start` arguments to `main` arguments
    fn c_main_args(args: RawSliceMut<NonNullSlice<u8>>) -> (i32, *const *const u8) {
        let argv_slice = unsafe { args.into_slice_mut().unwrap_or_default() };
        if argv_slice.is_empty() {
            return (0, core::ptr::null());
        }

        let bytes = (args.len() + 1) * size_of::<usize>();

        let c_argv_bytes = GLOBAL_SYSTEM_ALLOCATOR.allocate(bytes).unwrap();
        let c_argv_slice = unsafe {
            core::slice::from_raw_parts_mut(c_argv_bytes.as_ptr() as *mut *const u8, args.len() + 1)
        };

        for (i, arg) in argv_slice.iter().enumerate() {
            c_argv_slice[i] = arg.as_ptr();
        }

        c_argv_slice[args.len()] = core::ptr::null();

        (args.len() as i32, c_argv_slice.as_ptr())
    }

    let (argc, argv) = c_main_args(args);
    let result = main(argc, argv);
    syscalls::process::exit(result as usize)
}
