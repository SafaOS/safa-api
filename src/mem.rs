use core::ptr::NonNull;

use safa_abi::{errors::ErrorStatus, mem::MemMapFlags};

use crate::{resource::Resource, syscalls};

/// A cleaner interface over [`syscalls::mem::map`].
///
/// Used to map memory in an iterator over the same configuration (hints and flags).
///
/// Construct a new MemoryMapper with [`MemoryMapper::new`] and then map N pages using [`Self::map_next`].
#[derive(Debug, Clone, Copy)]
pub struct MemoryMapper {
    flags: MemMapFlags,
    hint: *const (),
    guard_pages: usize,
}

impl MemoryMapper {
    /// Constructs a new Memory Mapper.
    ///
    /// By default there are no hints, and the flags has all permissions (read write and execute).
    pub const fn new() -> Self {
        Self {
            flags: MemMapFlags::WRITE,
            hint: core::ptr::null(),
            guard_pages: 0,
        }
    }

    /// Builder pattern for setting flags.
    pub const fn flags(self, flags: MemMapFlags) -> Self {
        Self { flags, ..self }
    }

    /// Builder pattern for setting guard pages count.
    pub const fn guard(self, pages: usize) -> Self {
        Self {
            guard_pages: pages,
            ..self
        }
    }

    /// Builder pattern for setting hint.
    pub const fn hint(self, hint: *const ()) -> Self {
        Self { hint, ..self }
    }

    /// Maps the next `n` bytes.
    ///
    /// Returns a resource that is when dropped, deallocates the memory, and a NonNull slice to the allocated memory.
    pub fn map_next(&self, n: usize) -> Result<(Resource, NonNull<[u8]>), ErrorStatus> {
        syscalls::mem::map(self.hint, n, self.guard_pages, None, None, self.flags)
            .map(|(ri, data)| unsafe { (Resource::from_raw(ri), data) })
    }
    /// Maps the next `n` bytes, so that they point to the memory mapped interface of `resource`.
    ///
    /// Returns a resource that is when dropped, deallocates the memory, and a NonNull slice to the allocated memory.
    pub fn map_next_resource(
        &self,
        n: usize,
        resource: &Resource,
        map_offset: Option<isize>,
    ) -> Result<(Resource, NonNull<[u8]>), ErrorStatus> {
        syscalls::mem::map(
            self.hint,
            n,
            self.guard_pages,
            Some(resource.ri()),
            map_offset,
            self.flags,
        )
        .map(|(ri, data)| unsafe { (Resource::from_raw(ri), data) })
    }
}
