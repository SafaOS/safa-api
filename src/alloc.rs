//! This module implements a high-level userspace allocator
//! which internally uses the [`crate::syscalls::syssbrk`] syscall
//! to allocate memory

use crate::raw::{NonNullSlice, Optional};

use super::syscalls;
use core::{alloc::GlobalAlloc, cell::UnsafeCell, ptr::NonNull};

#[derive(Debug, Default)]
struct Block {
    free: bool,
    next: Option<NonNull<Block>>,
    data_len: usize,
}

impl Block {
    /// Asks the system for a new memory Block with a size big enough to hold `data_len` bytes
    pub fn create(data_len: usize) -> Option<NonNull<Self>> {
        let size = data_len + size_of::<Block>();
        let size = size.next_multiple_of(align_of::<Block>());
        assert!(size <= isize::MAX as usize);

        let ptr = get_data_break() as *mut Block;
        syscalls::sbrk(size as isize).ok()?;
        unsafe {
            *ptr = Self {
                free: true,
                data_len: size - size_of::<Block>(),
                ..Default::default()
            };
            Some(NonNull::new_unchecked(ptr))
        }
    }

    #[inline(always)]
    /// Gets the Block metadata of a data ptr,
    /// unsafe because the pointer had to be made by calling `[Block::data_from_ptr]` on a vaild pointer, otherwise the returned value is invaild
    pub unsafe fn block_from_data_ptr(data: NonNull<u8>) -> NonNull<Self> {
        unsafe { NonNull::new_unchecked((data.as_ptr() as *mut Block).offset(-1)) }
    }

    #[inline(always)]
    /// Gets the data ptr from a pointer to Block
    pub unsafe fn data_from_ptr(ptr: *const Self) -> NonNull<[u8]> {
        unsafe {
            let length = (*ptr).data_len;
            let ptr_to_data = ptr.offset(1) as *const u8 as *mut u8;
            NonNull::new_unchecked(core::slice::from_raw_parts_mut(ptr_to_data, length) as *mut [u8])
        }
    }
}

pub struct SystemAllocator {
    head: Option<NonNull<Block>>,
}

fn get_data_break() -> *mut u8 {
    // Should never fail
    unsafe { syscalls::sbrk(0).unwrap_unchecked() }
}

impl SystemAllocator {
    const fn new() -> Self {
        Self { head: None }
    }

    /// tries to find a block with enough space for `data_len` bytes
    #[inline]
    fn try_find_block(&self, data_len: usize) -> Option<NonNull<Block>> {
        // To optimize the search for exact size we have to manipulate the data_len a bit
        let size = data_len + size_of::<Block>();
        let size = size.next_multiple_of(align_of::<Block>());
        let data_len = size - size_of::<Block>();

        let mut current = self.head;
        let mut best_block: Option<(NonNull<Block>, usize)> = None;

        while let Some(block_ptr) = current {
            let block = unsafe { &*block_ptr.as_ptr() };
            if !block.free {
                current = block.next;
                continue;
            }

            if block.data_len == data_len {
                return Some(block_ptr);
            }

            if block.data_len > data_len
                && best_block.is_none_or(|(_, bb_len)| bb_len > block.data_len)
            {
                best_block = Some((block_ptr, block.data_len));
            }

            current = block.next;
        }

        best_block.map(|(ptr, _)| ptr)
    }

    /// finds a block with enough space for `data_len` bytes
    /// or creates a new one if there is no enough space
    #[inline]
    fn find_block(&mut self, data_len: usize) -> Option<NonNull<Block>> {
        if let Some(block) = self.try_find_block(data_len) {
            Some(block)
        } else {
            unsafe {
                let new_block = Block::create(data_len)?;
                let stolen_head = self.head.take();

                (*new_block.as_ptr()).next = stolen_head;
                self.head = Some(new_block);

                Some(new_block)
            }
        }
    }

    fn allocate(&mut self, size: usize) -> Option<NonNull<[u8]>> {
        let block = self.find_block(size)?;
        unsafe {
            let ptr = block.as_ptr();
            (*ptr).free = false;
            Some(Block::data_from_ptr(ptr))
        }
    }

    unsafe fn deallocate(&mut self, block_data: NonNull<u8>) {
        unsafe {
            let block_ptr = Block::block_from_data_ptr(block_data).as_ptr();
            let block = &mut *block_ptr;
            block.free = true;
        }
    }
}

unsafe impl Send for SystemAllocator {}
unsafe impl Sync for SystemAllocator {}

// FIXME: implement locks before multithreading
pub struct GlobalSystemAllocator {
    inner: UnsafeCell<SystemAllocator>,
}

impl GlobalSystemAllocator {
    const fn new() -> Self {
        Self {
            inner: UnsafeCell::new(SystemAllocator::new()),
        }
    }
    #[inline]
    pub fn allocate(&self, size: usize) -> Option<NonNull<[u8]>> {
        unsafe { (*self.inner.get()).allocate(size) }
    }
    #[inline]
    pub unsafe fn deallocate(&self, ptr: NonNull<u8>) {
        unsafe { (*self.inner.get()).deallocate(ptr) }
    }

    // TODO: implement grow and shrink
}

unsafe impl Sync for GlobalSystemAllocator {}
unsafe impl Send for GlobalSystemAllocator {}

unsafe impl GlobalAlloc for GlobalSystemAllocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        self.allocate(layout.size())
            .map(|x| x.as_ptr() as *mut u8)
            .unwrap_or(core::ptr::null_mut())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _: core::alloc::Layout) {
        self.deallocate(NonNull::new_unchecked(ptr));
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
pub extern "C" fn syscreate(object_size: usize) -> Optional<NonNullSlice<u8>> {
    GLOBAL_SYSTEM_ALLOCATOR
        .allocate(object_size)
        .map(|mut x| unsafe {
            let x = x.as_mut();
            let len = x.len();
            let ptr = NonNull::new_unchecked(x.as_mut_ptr());
            NonNullSlice::from_raw_parts(ptr, len)
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
