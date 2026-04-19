//! SafaOS Shared Memory IPC Wrappers.

use core::ptr::NonNull;

use safa_abi::errors::ErrorStatus;

use crate::{
    abi::mem::ShmFlags,
    mem::MemoryMapper,
    resource::Resource,
    syscalls::{self},
};

pub use crate::syscalls::mem::ShmKey;

/// higher-level wrapper around [`syscalls::mem::shm_create`].
pub fn raw_create(pages: usize, flags: ShmFlags) -> Result<(ShmKey, Resource), ErrorStatus> {
    syscalls::mem::shm_create(pages, flags)
        .map(|(key, resource)| (key, unsafe { Resource::from_raw(resource) }))
}

/// higher-level wrapper around [`syscalls::mem::shm_open`].
pub fn raw_open(key: ShmKey, flags: ShmFlags) -> Result<Resource, ErrorStatus> {
    syscalls::mem::shm_open(key, flags).map(|ri| unsafe { Resource::from_raw(ri) })
}

/// SharedObject represents a shared memory object between multiple address spaces.
///
/// Synchronization is left to be handled by the user.
#[derive(Debug)]
pub struct SharedObject {
    _shm: Resource,
    _mem_map: Resource,
    key: ShmKey,
    buf: NonNull<[u8]>,
}

unsafe impl Send for SharedObject {}
unsafe impl Sync for SharedObject {}

impl SharedObject {
    /// Maps and opens a given SHM key.
    ///
    /// returning a SharedObject on success.
    pub fn map_open(
        mem_mapper: &MemoryMapper,
        key: ShmKey,
        size: usize,
    ) -> Result<Self, crate::errors::ErrorStatus> {
        let shm_res = syscalls::mem::shm_open(key, ShmFlags::NONE)
            .map(|ri| unsafe { Resource::from_raw(ri) })?;

        let (mem_map, buf) = mem_mapper.map_next_resource(size.div_ceil(4096), &shm_res, None)?;
        Ok(Self {
            key,
            _mem_map: mem_map,
            _shm: shm_res,
            buf,
        })
    }
    /// Allocates a new shared memory object with the given size, shared with the WM.
    ///
    /// Returns a Result containing the SharedObject or an ErrorStatus if allocation fails.
    pub fn allocate(size: usize) -> Result<Self, crate::errors::ErrorStatus> {
        let pages = size.div_ceil(4096);
        let flags = ShmFlags::NONE;

        let (key, shm) = raw_create(pages, flags).expect("Failed to open ShmKey");
        let (mem_map, buf) = MemoryMapper::new().map_next_resource(pages, &shm, None)?;

        Ok(Self {
            key,
            _mem_map: mem_map,
            _shm: shm,
            buf,
        })
    }

    /// Returns the key of the shared memory object.
    pub fn shm_key(&self) -> ShmKey {
        self.key
    }

    /// Returns the pointer to the shared memory buffer.
    pub const fn data_ptr(&self) -> NonNull<[u8]> {
        self.buf
    }

    #[inline(always)]
    /// Returns the pointer to the shared memory buffer as a given Type T.
    pub const fn data_as<T: Sized>(&self) -> NonNull<[T]> {
        NonNull::slice_from_raw_parts(self.buf.cast::<T>(), self.buf.len() / size_of::<T>())
    }

    /// Returns a reference to the shared memory buffer.
    ///
    /// # Safety:
    /// Synchorization should be done between the memory-spaces that share that memory, as by using IPC and such.
    #[inline(always)]
    pub const unsafe fn data(&self) -> &[u8] {
        unsafe { self.buf.as_ref() }
    }
    /// Returns a muttable reference to the shared memory buffer.
    /// # Safety:
    /// Synchorization should be done between the memory-spaces that share that memory, as by using IPC and such.
    #[inline(always)]
    pub const unsafe fn data_mut(&mut self) -> &mut [u8] {
        unsafe { self.buf.as_mut() }
    }
}
