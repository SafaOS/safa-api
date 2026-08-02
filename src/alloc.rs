pub mod dumb;
#[cfg(feature = "talc")]
pub mod talc;

#[cfg(feature = "dlmalloc")]
pub mod dlmalloc;

use core::alloc::GlobalAlloc;
use core::ptr::NonNull;

#[cfg(not(any(feature = "talc", feature = "dlmalloc")))]
pub use dumb::*;
#[cfg(feature = "talc")]
pub use talc::*;

#[cfg(feature = "dlmalloc")]
pub use dlmalloc::*;

pub(crate) const KIB_1: usize = 1024;
pub(crate) const KIB_64: usize = 64 * KIB_1;
pub(crate) const KIB_4: usize = 4 * KIB_1;

pub(crate) const PAGE_SIZE: usize = KIB_4;
pub(crate) const LARGE_ALLOC_THRESHOLD: usize = KIB_64;

#[cfg(not(any(feature = "std", feature = "rustc-dep-of-std")))]
use safa_abi::ffi::{option::OptZero, slice::Slice};

unsafe impl GlobalAlloc for GlobalSystemAllocator {
    #[inline]
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        self.allocate(layout.size(), layout.align())
            .map(|x| x.as_ptr() as *mut u8)
            .unwrap_or(core::ptr::null_mut())
    }

    #[inline]
    unsafe fn dealloc(&self, ptr: *mut u8, _: core::alloc::Layout) {
        unsafe { self.deallocate(NonNull::new_unchecked(ptr)) };
    }

    #[inline]
    unsafe fn alloc_zeroed(&self, layout: core::alloc::Layout) -> *mut u8 {
        self.allocate_zeroed(layout.size(), layout.align())
            .map(|x| x.as_ptr() as *mut u8)
            .unwrap_or(core::ptr::null_mut())
    }
}

#[cfg_attr(
    not(any(feature = "std", feature = "rustc-dep-of-std")),
    global_allocator
)]
/// A high-level userspace allocator that internally uses the [`crate::syscalls::syssbrk`] syscall
/// (rust wrapper)
pub static GLOBAL_SYSTEM_ALLOCATOR: GlobalSystemAllocator = GlobalSystemAllocator::new();

#[cfg(not(any(feature = "std", feature = "rustc-dep-of-std")))]
#[unsafe(no_mangle)]
/// Allocates an object sized `object_size` using [`GLOBAL_SYSTEM_ALLOCATOR`]
pub extern "C" fn syscreate(object_size: usize, object_align: usize) -> OptZero<Slice<u8>> {
    GLOBAL_SYSTEM_ALLOCATOR
        .allocate(object_size, object_align)
        .map(|mut x| unsafe {
            let x = x.as_mut();
            Slice::from_raw_parts(x.as_mut_ptr(), x.len())
        })
        .into()
}

#[cfg(not(any(feature = "std", feature = "rustc-dep-of-std")))]
#[unsafe(no_mangle)]
/// Deallocates an object sized `object_size` using [`GLOBAL_SYSTEM_ALLOCATOR`]
/// # Safety
/// `object_ptr` must be a pointer to a valid object allocated by [`GLOBAL_SYSTEM_ALLOCATOR`]
pub unsafe extern "C" fn sysdestroy(object_ptr: *mut u8) {
    unsafe {
        match NonNull::new(object_ptr) {
            Some(ptr) => GLOBAL_SYSTEM_ALLOCATOR.deallocate(ptr),
            None => (),
        }
    }
}
