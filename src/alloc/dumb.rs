//! This module implements a high-level userspace allocator
//! which internally uses the [`crate::syscalls::mmap`] syscall
//! to allocate memory

use safa_abi::mem::MemFlags;

use crate::{
    alloc::{LARGE_ALLOC_THRESHOLD, PAGE_SIZE},
    mem::MemoryMapper,
    sync::locks::Mutex,
    syscalls,
};

use core::ptr::NonNull;

#[derive(Debug, Default)]
struct Block {
    free: bool,
    next: Option<NonNull<Block>>,
    data_len: usize,
    __padding: usize,
}

/// Memory is guaranteed to be zeroed.
fn sys_allocate(size_hint: usize) -> Option<NonNull<[u8]>> {
    let data = MemoryMapper::new()
        .prot(MemFlags::READ | MemFlags::WRITE | MemFlags::EXEC)
        .map_next_bytes(size_hint)
        .ok()?;
    Some(data)
}

impl Block {
    /// Asks the system for a new memory Block with a size big enough to hold `data_len` bytes
    pub fn create(
        data_len: usize,
        data_alignment: usize,
    ) -> Option<(NonNull<Self>, Option<NonNull<Block>>)> {
        let data_len = data_len.next_multiple_of(size_of::<Block>());
        let size = data_len + size_of::<Block>();
        let size = size
            .next_multiple_of(align_of::<Block>())
            .next_multiple_of(data_alignment);

        assert!(size <= isize::MAX as usize);

        let alloced = sys_allocate(size)?;
        let alloc_size = alloced.len();
        let alloc_ptr = alloced.as_ptr().cast::<u8>();

        assert!(alloc_size >= size);

        unsafe {
            // We want the data pointer to be aligned to the data alignment
            let data_ptr =
                (alloc_ptr.add(size_of::<Block>()) as usize).next_multiple_of(data_alignment);
            let ptr = (data_ptr - size_of::<Block>()) as *mut Block;

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
        let size = size.next_multiple_of(alignment);
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
    fn find_block(&mut self, data_len: usize, alignment: usize) -> Option<(NonNull<Block>, bool)> {
        assert!(
            alignment <= PAGE_SIZE,
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
            Some((block, false))
        } else {
            unsafe {
                let (new_block, new_allocation_tail) = Block::create(data_len, alignment)?;
                let set_next_of = new_allocation_tail.unwrap_or(new_block);
                let stolen_head = self.head.take();

                (*set_next_of.as_ptr()).next = stolen_head;
                self.head = Some(new_block);

                Some((new_block, true))
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

    fn large_allocate(&mut self, size: usize) -> Option<NonNull<[u8]>> {
        let alloc_size = (size + size_of::<Block>()).next_multiple_of(PAGE_SIZE);

        let mut block = sys_allocate(alloc_size)?;
        let mut block_ptr = block.cast::<Block>();

        assert_eq!(
            block.len(),
            alloc_size,
            "Failed to predict large allocation size"
        );
        let data_len = block.len() - size_of::<Block>();

        unsafe {
            block_ptr.as_mut().free = true;
            block_ptr.as_mut().data_len = data_len;
            block_ptr.as_mut().next = None;

            Some(NonNull::from_mut(&mut block.as_mut()[size_of::<Block>()..]))
        }
    }

    fn large_deallocate(&mut self, mut block: NonNull<Block>) {
        let block_mut = unsafe { block.as_mut() };

        let full_size = block_mut.data_len + size_of::<Block>();
        assert!(
            full_size.is_multiple_of(PAGE_SIZE),
            "large allocation isn't an exact multiple of pages"
        );
        assert!(
            (block.as_ptr() as usize).is_multiple_of(PAGE_SIZE),
            "Large allocation pointer isn't page aligned"
        );

        unsafe {
            syscalls::mem::unmap(block.as_ptr().cast(), full_size)
                .expect("Failed to deallocate large allocation")
        }
    }

    fn allocate(&mut self, zeroed: bool, size: usize, alignment: usize) -> Option<NonNull<[u8]>> {
        if size >= LARGE_ALLOC_THRESHOLD {
            return self.large_allocate(size);
        }

        let (block, new_block) = self.find_block(size, alignment)?;
        unsafe {
            let ptr = block.as_ptr();
            (*ptr).free = false;

            let data_ptr = Block::data_from_ptr(ptr);
            if zeroed && !new_block {
                data_ptr.cast::<u8>().write_bytes(0, data_ptr.len());
            }
            Some(data_ptr)
        }
    }

    unsafe fn deallocate_block(&mut self, mut block_ptr: NonNull<Block>) {
        unsafe {
            let block = block_ptr.as_mut();
            block.free = true;

            if block.data_len >= LARGE_ALLOC_THRESHOLD {
                return self.large_deallocate(block_ptr);
            }
            self.merge_blocks();
        }
    }
    unsafe fn deallocate(&mut self, block_data: NonNull<u8>) {
        unsafe {
            let block_ptr = Block::block_from_data_ptr(block_data);
            self.deallocate_block(block_ptr);
        }
    }

    #[inline]
    unsafe fn reallocate(
        &mut self,
        old_data: NonNull<u8>,
        new_size: usize,
        new_alignment: usize,
    ) -> Option<NonNull<[u8]>> {
        unsafe {
            let mut old_block_ptr = Block::block_from_data_ptr(old_data);
            let old_block = old_block_ptr.as_mut();

            if old_block.data_len >= new_size
                && (old_data.as_ptr() as usize).is_multiple_of(new_alignment)
            {
                return Some(Block::data_from_ptr(old_block));
            }

            let new_data = self.allocate(false, new_size, new_alignment)?;

            new_data
                .cast()
                .copy_from_nonoverlapping(old_data, old_block.data_len.min(new_data.len()));

            self.deallocate_block(old_block_ptr);
            Some(new_data)
        }
    }
}

unsafe impl Send for SystemAllocator {}
unsafe impl Sync for SystemAllocator {}

pub struct GlobalSystemAllocator {
    inner: Mutex<SystemAllocator>,
}

impl GlobalSystemAllocator {
    pub const fn new() -> Self {
        Self {
            inner: Mutex::new(SystemAllocator::new()),
        }
    }

    #[inline]
    pub fn allocate(&self, size: usize, alignment: usize) -> Option<NonNull<[u8]>> {
        self.inner.lock().allocate(false, size, alignment)
    }

    #[inline]
    pub fn allocate_zeroed(&self, size: usize, alignment: usize) -> Option<NonNull<[u8]>> {
        self.inner.lock().allocate(true, size, alignment)
    }

    #[inline]
    pub unsafe fn deallocate(&self, ptr: NonNull<u8>) {
        unsafe { self.inner.lock().deallocate(ptr) }
    }

    #[inline]
    pub unsafe fn reallocate(
        &self,
        ptr: NonNull<u8>,
        new_size: usize,
        align: usize,
    ) -> Option<NonNull<[u8]>> {
        unsafe { self.inner.lock().reallocate(ptr, new_size, align) }
    }

    // TODO: implement proper grow and shrink
}

unsafe impl Sync for GlobalSystemAllocator {}
unsafe impl Send for GlobalSystemAllocator {}
