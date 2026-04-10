//! Shared memory objects used by the compositor.

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
