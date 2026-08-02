use core::{alloc::Layout, ptr::NonNull};

use safa_abi::mem::MemFlags;
use talc::{TalcLock, source::Source};

use crate::{mem::MemoryMapper, sync};

struct Header {
    size: usize,
    align: usize,
}

impl Header {
    pub const fn size_with_align(align: usize) -> usize {
        size_of::<Self>().next_multiple_of(align)
    }

    pub const fn off_with_align(align: usize) -> usize {
        Self::size_with_align(align) - size_of::<Self>()
    }

    pub const unsafe fn from_data_ptr(ptr: NonNull<u8>) -> (NonNull<Header>, NonNull<[u8]>) {
        let header = unsafe { ptr.byte_sub(size_of::<Self>()).cast::<Header>() };
        let data = unsafe { NonNull::slice_from_raw_parts(ptr, header.as_ref().size) };

        (header, data)
    }

    pub unsafe fn alloc_ptr_from_header(header: NonNull<Header>) -> (NonNull<u8>, Layout) {
        unsafe {
            let size_of_header = Header::size_with_align(header.as_ref().align);
            let layout = Layout::from_size_align(
                header.as_ref().size + size_of_header,
                header.as_ref().align,
            )
            .expect("Failed to get layout from header");
            (
                header
                    .byte_sub(Self::off_with_align(header.as_ref().align))
                    .cast(),
                layout,
            )
        }
    }

    pub const unsafe fn claim_allocation(
        alloc_ptr: NonNull<u8>,
        alloc_size: usize,
        alloc_align: usize,
    ) -> (NonNull<Header>, NonNull<[u8]>) {
        let our_size = Self::size_with_align(alloc_align);

        let data_size = alloc_size - our_size;
        let data_header = unsafe {
            alloc_ptr
                .byte_add(Self::off_with_align(alloc_align))
                .cast::<Header>()
        };
        let data =
            unsafe { NonNull::slice_from_raw_parts(alloc_ptr.byte_add(our_size), data_size) };

        (data_header, data)
    }
}

/// Memory is guaranteed to be zeroed.
fn sys_allocate(size_hint: usize) -> Option<NonNull<[u8]>> {
    let data = MemoryMapper::new()
        .prot(MemFlags::READ | MemFlags::WRITE | MemFlags::EXEC)
        .map_next_bytes(size_hint)
        .ok()?;
    Some(data)
}

#[derive(Debug)]
struct MMapSource {
    last_allocation_end: Option<NonNull<u8>>,
}

unsafe impl Source for MMapSource {
    fn acquire<B: talc::base::binning::Binning>(
        talc: &mut talc::base::Talc<Self, B>,
        layout: core::alloc::Layout,
    ) -> Result<(), ()> {
        let mem = sys_allocate(layout.size()).ok_or(())?;
        let mem_ptr: NonNull<u8> = mem.cast();

        let last_end = talc.source.last_allocation_end;
        let current_end = unsafe { mem_ptr.byte_add(mem.len()) };

        talc.source.last_allocation_end = Some(current_end);

        unsafe {
            if let Some(last_end) = last_end
                && last_end == mem_ptr
            {
                talc.extend(last_end, current_end.as_ptr());
            } else {
                talc.claim(mem.as_ptr().cast(), mem.len())
                    .expect("Failed to claim another heap");
            }
        }

        Ok(())
    }
}

pub struct GlobalSystemAllocator(TalcLock<sync::MutexInner, MMapSource>);

impl GlobalSystemAllocator {
    pub fn allocate_with_layout(&self, layout: Layout) -> Option<NonNull<[u8]>> {
        assert!(layout.align() >= align_of::<Header>());
        assert!(layout.size() >= Header::size_with_align(layout.align()));

        unsafe {
            let alloc_ptr = self.0.lock().allocate(layout)?;
            let (mut alloc_header, alloc_data) =
                Header::claim_allocation(alloc_ptr, layout.size(), layout.align());

            alloc_header.as_mut().size = alloc_data.len();
            alloc_header.as_mut().align = layout.align();

            Some(alloc_data)
        }
    }
    pub fn allocate(&self, size: usize, align: usize) -> Option<NonNull<[u8]>> {
        let alloc_align = align_of::<Header>().max(align).next_power_of_two();

        let alloc_size = size + Header::size_with_align(alloc_align);
        let layout = Layout::from_size_align(alloc_size, alloc_align)
            .expect("Failed to get layout for allocation");

        self.allocate_with_layout(layout)
    }

    pub fn allocate_zeroed(&self, size: usize, align: usize) -> Option<NonNull<[u8]>> {
        let data = self.allocate(size, align)?;

        unsafe { data.cast::<NonNull<u8>>().write_bytes(0, data.len()) };
        Some(data)
    }

    pub unsafe fn deallocate(&self, ptr: NonNull<u8>) {
        unsafe {
            let (header, _) = Header::from_data_ptr(ptr);
            let (alloc_ptr, layout) = Header::alloc_ptr_from_header(header);
            self.0.lock().deallocate(alloc_ptr.as_ptr(), layout);
        }
    }

    pub unsafe fn reallocate(
        &self,
        ptr: NonNull<u8>,
        new_size: usize,
        new_align: usize,
    ) -> Option<NonNull<[u8]>> {
        let alloc_align = align_of::<Header>().max(new_align).next_power_of_two();

        let alloc_size = new_size + Header::size_with_align(alloc_align);
        let new_layout = Layout::from_size_align(alloc_size, alloc_align)
            .expect("Failed to get layout for allocation");

        unsafe {
            let (header, old_data) = Header::from_data_ptr(ptr);
            let (alloc_ptr, old_layout) = Header::alloc_ptr_from_header(header);

            if old_layout.align() == new_layout.align() {
                if self.0.lock().try_realloc_in_place(
                    alloc_ptr.as_ptr(),
                    old_layout,
                    new_layout.size(),
                ) {
                    let (mut new_alloc_header, new_alloc_data) =
                        Header::claim_allocation(alloc_ptr, new_layout.size(), new_layout.align());

                    new_alloc_header.as_mut().size = new_alloc_data.len();
                    new_alloc_header.as_mut().align = new_layout.align();

                    return Some(new_alloc_data);
                }
            }

            let new_data = self.allocate_with_layout(new_layout)?;
            new_data.cast::<u8>().copy_from_nonoverlapping(
                old_data.cast::<u8>(),
                old_data.len().min(new_data.len()),
            );

            self.deallocate(ptr);
            Some(new_data)
        }
    }

    pub const fn new() -> Self {
        Self(TalcLock::new(MMapSource {
            last_allocation_end: None,
        }))
    }
}

unsafe impl Send for GlobalSystemAllocator {}
unsafe impl Sync for GlobalSystemAllocator {}
