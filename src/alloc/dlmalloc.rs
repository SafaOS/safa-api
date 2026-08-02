use core::ptr::NonNull;

use dlmalloc::{Allocator, Dlmalloc};
use safa_abi::mem::{MemFlags, MemMap2Flags};

use crate::{alloc::PAGE_SIZE, mem::MemoryMapper, sync::Mutex};

struct MMapSystem;
unsafe impl Allocator for MMapSystem {
    fn allocates_zeros(&self) -> bool {
        true
    }

    fn alloc(&self, size: usize) -> (*mut u8, usize, u32) {
        let Ok(bytes) = MemoryMapper::new()
            .prot(MemFlags::READ | MemFlags::WRITE | MemFlags::EXEC)
            .map_next_bytes(size)
        else {
            return (core::ptr::null_mut(), 0, 0);
        };

        (bytes.as_ptr().cast(), bytes.len(), 0)
    }

    fn page_size(&self) -> usize {
        PAGE_SIZE
    }

    fn remap(&self, ptr: *mut u8, oldsize: usize, newsize: usize, can_move: bool) -> *mut u8 {
        if oldsize >= newsize || oldsize.div_ceil(PAGE_SIZE) == newsize.div_ceil(PAGE_SIZE) {
            return ptr;
        }

        let diff = newsize - oldsize;
        let end = unsafe { ptr.byte_add(oldsize.next_multiple_of(PAGE_SIZE)) };

        let attempt_inplace = MemoryMapper::new()
            .prot(MemFlags::READ | MemFlags::WRITE | MemFlags::EXEC)
            .flags(MemMap2Flags::FIXED)
            .hint(end.cast())
            .map_next_bytes(diff);

        if let Ok(_) = attempt_inplace {
            return ptr;
        }

        if !can_move {
            return core::ptr::null_mut();
        }

        let (new_ptr, _, _) = self.alloc(newsize);
        unsafe { new_ptr.copy_from(ptr, oldsize) };

        unsafe {
            crate::syscalls::mem::unmap(ptr.cast(), oldsize).expect("Failed to unmap old memory")
        };
        new_ptr
    }

    fn free(&self, ptr: *mut u8, size: usize) -> bool {
        unsafe {
            crate::syscalls::mem::unmap(ptr.cast(), size).expect("Failed to unmap old memory")
        };

        true
    }

    fn free_part(&self, ptr: *mut u8, oldsize: usize, newsize: usize) -> bool {
        let diff = oldsize - newsize;
        let free_start = unsafe { ptr.byte_add(newsize) };
        if (free_start as usize).is_multiple_of(PAGE_SIZE) && diff.is_multiple_of(PAGE_SIZE) {
            unsafe {
                crate::syscalls::mem::unmap(free_start.cast(), diff)
                    .expect("Failed to unmap freed memory");
            }

            return true;
        }
        false
    }

    fn can_release_part(&self, _flags: u32) -> bool {
        true
    }
}

pub struct GlobalSystemAllocator(Mutex<Dlmalloc<MMapSystem>>);

impl GlobalSystemAllocator {
    pub const fn new() -> Self {
        GlobalSystemAllocator(Mutex::new(Dlmalloc::<MMapSystem>::new_with_allocator(
            MMapSystem,
        )))
    }

    pub fn allocate(&self, size: usize, align: usize) -> Option<NonNull<[u8]>> {
        NonNull::new(unsafe { self.0.lock().malloc(size, align) })
            .map(|n| NonNull::slice_from_raw_parts(n, size))
    }

    pub fn allocate_zeroed(&self, size: usize, align: usize) -> Option<NonNull<[u8]>> {
        NonNull::new(unsafe { self.0.lock().calloc(size, align) })
            .map(|n| NonNull::slice_from_raw_parts(n, size))
    }

    #[inline]
    pub unsafe fn deallocate(&self, ptr: NonNull<u8>) {
        unsafe { self.0.lock().c_free(ptr.as_ptr()) }
    }

    #[inline]
    pub unsafe fn reallocate(
        &self,
        ptr: NonNull<u8>,
        new_size: usize,
        _align: usize,
    ) -> Option<NonNull<[u8]>> {
        NonNull::new(unsafe { self.0.lock().c_realloc(ptr.as_ptr(), new_size) })
            .map(|n| NonNull::slice_from_raw_parts(n, new_size))
    }
}
