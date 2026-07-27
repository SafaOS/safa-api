use core::ptr::NonNull;

use safa_abi::{
    errors::ErrorStatus,
    mem::{MemFlags, MemMap2Flags},
};

use crate::{resource::Resource, syscalls};

/// A cleaner interface over [`syscalls::mem::map`].
///
/// Used to map memory in an iterator over the same configuration (hints and flags).
///
/// Construct a new MemoryMapper with [`MemoryMapper::new`] and then map N pages using [`Self::map_next`].
#[derive(Debug, Clone, Copy)]
pub struct MemoryMapper {
    flags: MemMap2Flags,
    prot: MemFlags,
    hint: *const (),
}

impl MemoryMapper {
    /// Constructs a new Memory Mapper.
    ///
    /// By default there are no hints, and the flags has all permissions (read write and execute).
    pub const fn new() -> Self {
        Self::from_prot(MemFlags::from_bits(
            MemFlags::READ.to_bits() | MemFlags::WRITE.to_bits() | MemFlags::EXEC.to_bits(),
        ))
    }

    pub const fn from_prot(prot: MemFlags) -> Self {
        Self {
            flags: MemMap2Flags::NONE,
            prot,
            hint: core::ptr::null(),
        }
    }

    /// Builder pattern for setting flags.
    pub const fn flags(self, flags: MemMap2Flags) -> Self {
        Self { flags, ..self }
    }

    /// Builder pattern for setting flags.
    pub const fn prot(self, prot: MemFlags) -> Self {
        Self { prot, ..self }
    }

    /// Builder pattern for setting hint.
    pub const fn hint(self, hint: *const ()) -> Self {
        Self { hint, ..self }
    }

    fn map_next_inner(
        &self,
        n: usize,
        ctrl: bool,
    ) -> Result<(Option<Resource>, NonNull<[u8]>), ErrorStatus> {
        let mut res_out = 0xAAAAAAAA;
        let ctrl_res = ctrl.then_some(&mut res_out);
        let data = syscalls::mem::map(self.hint, n, None, None, self.flags, self.prot, ctrl_res)?;

        Ok((ctrl.then(|| unsafe { Resource::from_raw(res_out) }), data))
    }

    /// Maps the next `n` bytes.
    ///
    /// Returns a resource that is when dropped, deallocates the memory, and a NonNull slice to the allocated memory.
    pub fn map_next_bytes(&self, n: usize) -> Result<NonNull<[u8]>, ErrorStatus> {
        self.map_next_inner(n, false).map(|(_, d)| d)
    }

    /// Same as [`Self::map_next_bytes`] but also creates a control resource that if dropped the memory is unmmapped.
    pub fn map_next_bytes_with_ctrl(
        &self,
        n: usize,
    ) -> Result<(Resource, NonNull<[u8]>), ErrorStatus> {
        self.map_next_inner(n, true).map(|(c, d)| (c.unwrap(), d))
    }

    /// Maps the next `n` bytes, so that they point to the memory mapped interface of `resource`.
    ///
    /// Returns a resource that is when dropped, deallocates the memory, and a NonNull slice to the allocated memory.
    pub fn map_next_resource_bytes(
        &self,
        n: usize,
        resource: &Resource,
        map_offset: Option<isize>,
    ) -> Result<(Resource, NonNull<[u8]>), ErrorStatus> {
        self.map_next_resource_inner(n, resource, map_offset, true)
            .map(|(r, d)| (r.unwrap(), d))
    }

    fn map_next_resource_inner(
        &self,
        n: usize,
        resource: &Resource,
        map_offset: Option<isize>,
        ctrl: bool,
    ) -> Result<(Option<Resource>, NonNull<[u8]>), ErrorStatus> {
        let mut res_out = 0xAAAAAAAA;
        let ctrl_res = ctrl.then_some(&mut res_out);
        let data = syscalls::mem::map(
            self.hint,
            n,
            Some(resource.ri()),
            map_offset,
            self.flags,
            self.prot,
            ctrl_res,
        )?;

        Ok((ctrl.then(|| unsafe { Resource::from_raw(res_out) }), data))
    }
}
