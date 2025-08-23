//! This module implements a high-level userspace allocator
//! which internally uses the [`crate::syscalls::syssbrk`] syscall
//! to allocate memory

#[cfg(not(any(feature = "std", feature = "rustc-dep-of-std")))]
use safa_abi::ffi::{option::OptZero, slice::Slice};
use safa_abi::mem::MemMapFlags;

use crate::sync::locks::Mutex;

use super::syscalls;
use core::{alloc::GlobalAlloc, ptr::NonNull};

#[derive(Debug, Default)]
struct Block {
    free: bool,
    next: Option<NonNull<Block>>,
    data_len: usize,
    __padding: usize,
}

fn sys_allocate(size_hint: usize) -> Option<(*mut u8, usize)> {
    let page_count = size_hint.next_multiple_of(4096) / 4096;
    let (_, s) = syscalls::mem::map(
        core::ptr::null(),
        page_count,
        0,
        None,
        None,
        MemMapFlags::WRITE,
    )
    .ok()?;

    Some((s.as_ptr() as *mut u8, s.len()))
}

impl Block {
    /// Asks the system for a new memory Block with a size big enough to hold `data_len` bytes
    pub fn create(data_len: usize) -> Option<(NonNull<Self>, Option<NonNull<Block>>)> {
        let data_len = data_len.next_multiple_of(size_of::<Block>());
        let size = data_len + size_of::<Block>();
        let size = size.next_multiple_of(align_of::<Block>());
        assert!(size <= isize::MAX as usize);

        let (alloc_ptr, alloc_size) = sys_allocate(size)?;
        assert!(alloc_size >= size);

        let ptr = alloc_ptr as *mut Block;

        unsafe {
            *ptr = Self {
                free: true,
                data_len: size - size_of::<Block>(),
                ..Default::default()
            };

            if alloc_size > size {
                let extra_block_size = alloc_size - size;
                let extra_block_ptr = alloc_ptr.add(size) as *mut Block;
                *extra_block_ptr = Self {
                    free: true,
                    data_len: extra_block_size - size_of::<Block>(),
                    ..Default::default()
                };

                (*ptr).next = Some(NonNull::new_unchecked(extra_block_ptr));
            }
            Some((NonNull::new_unchecked(ptr), (*ptr).next))
        }
    }

    #[inline(always)]
    /// Gets the Block metadata of a data ptr,
    /// unsafe because the pointer had to be made by calling `[Block::data_from_ptr]` on a valid pointer, otherwise the returned value is invalid
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

impl SystemAllocator {
    const fn new() -> Self {
        Self { head: None }
    }

    /// tries to find a block with enough space for `data_len` bytes
    #[inline]
    fn try_find_block(&self, data_len: usize, alignment: usize) -> Option<NonNull<Block>> {
        let alignment = alignment.next_multiple_of(align_of::<Block>());

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

            if unsafe {
                !(Block::data_from_ptr(block).cast::<u8>().as_ptr() as usize)
                    .is_multiple_of(alignment)
            } {
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
    fn find_block(&mut self, data_len: usize, alignment: usize) -> Option<NonNull<Block>> {
        assert!(
            alignment <= 4096,
            "Max allowed alignment is Page size which is 4096 bytes"
        );

        let data_len = data_len.next_multiple_of(size_of::<Block>());

        if let Some(block) = self.try_find_block(data_len, alignment) {
            let block_ptr = block.as_ptr();

            unsafe {
                let block_len = (*block_ptr).data_len;

                // Spilt the Block
                if block_len > data_len && (block_len - data_len) > size_of::<Block>() {
                    let left_over = block_len - data_len;
                    let new_block_len = left_over - size_of::<Block>();

                    let new_block = block_ptr.add(1).byte_add(data_len);
                    *new_block = Block {
                        free: true,
                        data_len: new_block_len,
                        next: (*block_ptr).next.take(),
                        __padding: 0,
                    };

                    (*block_ptr).next = Some(NonNull::new_unchecked(new_block));
                    (*block_ptr).data_len = data_len;
                }
            }
            Some(block)
        } else {
            unsafe {
                let (new_block, new_allocation_tail) = Block::create(data_len)?;
                let set_next_of = new_allocation_tail.unwrap_or(new_block);
                let stolen_head = self.head.take();

                (*set_next_of.as_ptr()).next = stolen_head;
                self.head = Some(new_block);

                Some(new_block)
            }
        }
    }

    fn merge_blocks(&mut self) {
        let mut current = self.head;
        while let Some(block_ptr) = current {
            unsafe {
                let block = block_ptr.as_ptr();
                if !(*block).free {
                    current = (*block).next;
                    continue;
                }

                let Some(next) = (*block).next else {
                    return;
                };

                let next_ptr = next.as_ptr();
                if !(*next_ptr).free {
                    current = (*block).next;
                    continue;
                }

                if block.add(1).byte_add((*block).data_len) == next_ptr {
                    // consume the next block
                    (*block).next = (*next_ptr).next;
                    (*block).data_len += (*next_ptr).data_len + size_of::<Block>();
                }

                current = (*block).next;
            }
        }
    }

    fn allocate(&mut self, size: usize, alignment: usize) -> Option<NonNull<[u8]>> {
        let block = self.find_block(size, alignment)?;
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

            self.merge_blocks();
        }
    }
}

unsafe impl Send for SystemAllocator {}
unsafe impl Sync for SystemAllocator {}

// FIXME: implement locks before multithreading
pub struct GlobalSystemAllocator {
    inner: Mutex<SystemAllocator>,
}

impl GlobalSystemAllocator {
    const fn new() -> Self {
        Self {
            inner: Mutex::new(SystemAllocator::new()),
        }
    }

    #[inline]
    pub fn allocate(&self, size: usize, alignment: usize) -> Option<NonNull<[u8]>> {
        self.inner.lock().allocate(size, alignment)
    }

    #[inline]
    pub unsafe fn deallocate(&self, ptr: NonNull<u8>) {
        self.inner.lock().deallocate(ptr)
    }

    // TODO: implement grow and shrink
}

unsafe impl Sync for GlobalSystemAllocator {}
unsafe impl Send for GlobalSystemAllocator {}

unsafe impl GlobalAlloc for GlobalSystemAllocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        self.allocate(layout.size(), layout.align())
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
