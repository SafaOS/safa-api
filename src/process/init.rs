//! contains api initialization functions, that should be called before using the api
use core::ptr::NonNull;

use safa_abi::{
    ffi::{slice::Slice, str::Str},
    process::AbiStructures,
};

use crate::{
    alloc::GLOBAL_SYSTEM_ALLOCATOR,
    exported_func,
    process::env::RawEnv,
    syscalls::{self},
};

use super::{
    args::{RawArgs, SAAPI_RAW_ARGS},
    env::SAAPI_RAW_ENV,
    stdio::init_meta,
};

// Initialization

fn init_args(args: Option<NonNull<[&'static str]>>) {
    unsafe {
        let raw = RawArgs::new(args);
        SAAPI_RAW_ARGS.init(raw)
    }
}

fn init_env(env: Option<NonNull<[&'static [u8]]>>) {
    unsafe {
        let raw = RawEnv::new(env);
        SAAPI_RAW_ENV.init(raw)
    }
}

exported_func! {
    /// Initializes the safa-api
    /// if your programs are designed as C main function,
    ///
    /// use [`_c_api_init`] instead
    pub extern "C" fn sysapi_init(
        args: Slice<Str>,
        env: Slice<Slice<u8>>,
        task_abi_structures: AbiStructures,
    ) {
        unsafe {
        let args = args.try_into_str_slices_mut(|_| true).expect("invalid args passed to sysapi_init");
        let args_ptr =  NonNull::new_unchecked(args as *mut [&'static str]) ;

        let env = env.try_into_slices_ptr_mut(|_| true).expect("invalid env passed to sysapi_init");
        let env_ptr =  NonNull::new_unchecked(env as *mut [&'static [u8]]) ;

        init_args(Some(args_ptr));
        init_env(Some(env_ptr));
        init_meta(task_abi_structures);
        }
    }
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
    args: Slice<Str>,
    env: Slice<Slice<u8>>,
    task_abi_structures: *const AbiStructures,
    main: extern "C" fn(argc: i32, argv: *const *const u8) -> i32,
) -> ! {
    sysapi_init(args, env, *task_abi_structures);

    // Convert SafaOS `_start` arguments to `main` arguments
    fn c_main_args(args: Slice<Str>) -> (i32, *const *const u8) {
        let argv_slice = unsafe {
            args.try_as_slice()
                .expect("argv passed to _c_api_init are invalid")
        };

        if argv_slice.is_empty() {
            return (0, core::ptr::null());
        }

        let bytes = (args.len() + 1) * size_of::<usize>();

        let c_argv_bytes = GLOBAL_SYSTEM_ALLOCATOR.allocate(bytes, 16).unwrap();
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
