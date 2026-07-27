use core::ptr::NonNull;

use safa_abi::errors::ErrorStatus;
use safa_abi::mem::{
    MemFlags, MemMap2Flags, MemMapFlags, MemMapOp, MemMapTarget, RawMemMap2Config, RawMemMapConfig,
    ShmFlags,
};

use crate::syscalls::types::{IntoSyscallArg, RequiredPtrMut, Ri};

use super::types::{OptionalPtrMut, RequiredPtr};
use super::SyscallNum;

impl IntoSyscallArg for MemMapFlags {
    type RegResults = (usize,);
    fn into_syscall_arg(self) -> Self::RegResults {
        (unsafe { core::mem::transmute::<_, u8>(self) } as usize,)
    }
}

impl IntoSyscallArg for MemMap2Flags {
    type RegResults = (usize,);
    fn into_syscall_arg(self) -> Self::RegResults {
        (unsafe { core::mem::transmute::<_, u32>(self) } as usize,)
    }
}

impl IntoSyscallArg for MemFlags {
    type RegResults = (usize,);
    fn into_syscall_arg(self) -> Self::RegResults {
        (unsafe { core::mem::transmute::<_, u8>(self) } as usize,)
    }
}

define_syscall! {
    SyscallNum::SysMemMap => {
        /// See [`SyscallNum::SysMemMap`]
        sysmem_map(memmap_config: RequiredPtr<RawMemMapConfig>, flags: MemMapFlags, out_res_id: OptionalPtrMut<Ri>) NonNull<u8>
    },
    SyscallNum::SysMemUnmap => {
        /// See [`SyscallNum::SysMemMap`]
        sysmem_unmap(addr: *const (), size: usize)
    },
    SyscallNum::SysMemOp => {
        sysmem_op(target: RequiredPtr<MemMapTarget>, op: RequiredPtr<MemMapOp>)
    },
    SyscallNum::SysMemMap2 => {
        /// See [`SyscallNum::SysMemMap`] and [`SyscallNum::SysMemMap2`].
        sysmem_map2(memmap_config: RequiredPtr<RawMemMap2Config>, flags: MemMap2Flags, prot: MemFlags, out_res_id: OptionalPtrMut<Ri>) NonNull<u8>
    },
}

/// See [`SyscallNum::SysMemMap`] and [`RawMemMapConfig`]
///
/// You don't have to provide the [`MemMapFlags::MAP_RESOURCE`] flag, it is automatically provided if `resource_to_map` is Some
/// # Returns
/// the resource ID of the Tracked Mapping and the slice of bytes in the mapping
pub fn map(
    addr_hint: *const (),
    size: usize,
    resource_to_map: Option<Ri>,
    resource_off: Option<isize>,
    mut flags: MemMap2Flags,
    prot: MemFlags,
    control_resource_out: Option<&mut Ri>,
) -> Result<NonNull<[u8]>, ErrorStatus> {
    let size = size.next_multiple_of(4096);

    let (ri, off) = if let Some(ri) = resource_to_map {
        let off = resource_off.unwrap_or_default();
        flags = flags | MemMap2Flags::MAP_RESOURCE;
        (ri, off)
    } else {
        (0, 0)
    };

    let conf = RawMemMap2Config::V0 {
        size,
        resource_off: off,
        resource_to_map: ri,
        __rsv0: 0,
        __rsvd1: 0,
        addr_hint,
    };

    let result_start_addr = unsafe {
        sysmem_map2(
            RequiredPtr::new_unchecked(&raw const conf as *mut _),
            flags,
            prot,
            if let Some(res) = control_resource_out {
                RequiredPtr::new(res).into()
            } else {
                OptionalPtrMut::none()
            },
        )
        .get()?
    };

    let slice = unsafe { core::slice::from_raw_parts_mut(result_start_addr.as_ptr(), size) };

    unsafe { Ok(NonNull::new_unchecked(slice)) }
}

/// Unmaps a given address with the given size.
pub unsafe fn unmap(addr: *const (), size: usize) -> Result<(), ErrorStatus> {
    sysmem_unmap(addr, size).get()
}

pub unsafe fn memop(mut target: MemMapTarget, mut op: MemMapOp) -> Result<(), ErrorStatus> {
    unsafe {
        sysmem_op(
            RequiredPtr::new_unchecked(&raw mut target),
            RequiredPtr::new_unchecked(&raw mut op),
        )
        .get()
    }
}

/// Changes the protection rules of the memory at the given `addr` with the given `size` to `rules`.
pub unsafe fn protect(addr: *const (), size: usize, rules: MemFlags) -> Result<(), ErrorStatus> {
    unsafe {
        memop(
            MemMapTarget::Direct(addr.cast(), size),
            MemMapOp::Protect(rules),
        )
    }
}

impl IntoSyscallArg for ShmFlags {
    type RegResults = (usize,);
    fn into_syscall_arg(self) -> Self::RegResults {
        let as_u32: u32 = unsafe { core::mem::transmute(self) };
        (as_u32 as usize,)
    }
}

/// A Shared Memory Descriptor Key, that can be opened using [`sysmem_shm_open`] or created using [`sysmem_shm_create`].
pub type ShmKey = usize;

define_syscall! {
    SyscallNum::SysMemShmCreate => {
        /// Create a Shared Memory Descriptor, returning a key that points to it,
        /// The life time of that descriptor is bound to the calling process or the thread if the flag [`ShmFlags::LOCAL`] was specified.
        ///
        /// The returned Key can then be opened from another process using [`sysmem_shm_open`] and then [`sysmem_map`]ped,
        /// instead of calling [`sysmem_shm_open`] afterwards this returns an Optional Resource ID that can be mapped directly using [`sysmem_map`] from the calling process,
        /// but the desired Process to communicate with, should use [`sysmem_shm_open`] to get it's own copy.
        ///
        /// The lifetime of the key is extended for each [`sysmem_shm_open`] call, so that it isn't dropped until all the threads/processes that owns it are dropped.
        /// # Arguments
        /// * `page_count` - The number of pages to allocate for the shared memory descriptor.
        /// * `flags` - The flags to use when creating and opening the shared memory descriptor.
        /// * `out_shm_key` - A pointer to a [`ShmKey`] that will be filled with the key of the created shared memory descriptor.
        /// # Returns
        /// * On Ok: The resource ID of the created shared memory descriptor, as if a call to [`sysmem_shm_open`] was made on the `out_shm_key`.
        sysmem_shm_create(page_count: usize, flags: ShmFlags, out_shm_key: RequiredPtrMut<ShmKey>) Ri
    },
    SyscallNum::SysMemShmOpen => {
        /// Creates a Resource that can be [`sysmem_map`]ped to a Shared Memory Descriptor,
        /// Takes in a key that was created using [`sysmem_shm_create`].
        ///
        /// The lifetime of the Resource is bound to the process or a single thread if the flag [`ShmFlags::LOCAL`] was specified.
        ///
        /// # Arguments
        /// * `shm_key` - The key of the shared memory descriptor to open.
        /// * `flags` - The flags to use when opening the shared memory descriptor.
        /// # Returns
        /// * On Ok: The resource ID of the opened shared memory descriptor.
        sysmem_shm_open(shm_key: ShmKey, flags: ShmFlags) Ri
    },
}

/// Create a Shared Memory Descriptor, returning a key that points to it,
/// The life time of that descriptor is bound to the calling process or the thread if the flag [`ShmFlags::LOCAL`] was specified.
///
/// The returned Key can then be opened from another process using [`shm_open`] and then [`map`]ped,
/// instead of calling [`shm_open`] afterwards this returns an Optional Resource ID that can be mapped directly using [`map`] from the calling process,
/// but the desired Process to communicate with, should use [`shm_open`] to get it's own copy.
///
/// The lifetime of the key is extended for each [`shm_open`] call, so that it isn't dropped until all the threads/processes that owns it are dropped.
/// # Arguments
/// * `page_count` - The number of pages to allocate for the shared memory descriptor.
/// * `flags` - The flags to use when creating and opening the shared memory descriptor.
/// # Returns
/// * `Ok((ShmKey, Ri))` - The key and a resource ID that is created as if it was made by a call to [`shm_open`].
/// * `Err(ErrorStatus)` - An error.
pub fn shm_create(page_count: usize, flags: ShmFlags) -> Result<(ShmKey, Ri), ErrorStatus> {
    let mut shm_key = ShmKey::default();
    unsafe {
        sysmem_shm_create(
            page_count,
            flags,
            RequiredPtrMut::new_unchecked(&mut shm_key),
        )
        .get()
        .map(|resource_id| (shm_key, resource_id))
    }
}

/// Creates a Resource that can be [`map`]ped to a Shared Memory Descriptor,
/// Takes in a key that was created using [`shm_create`].
///
/// The lifetime of the Resource is bound to the process or a single thread if the flag [`ShmFlags::LOCAL`] was specified.
///
/// # Arguments
/// * `shm_key` - The key of the shared memory descriptor to open.
/// * `flags` - The flags to use when opening the shared memory descriptor.
/// # Returns
/// * `Ok(Ri)` - The resource ID of the opened shared memory descriptor.
/// * `Err(ErrorStatus)` - An error.
pub fn shm_open(shm_key: ShmKey, flags: ShmFlags) -> Result<Ri, ErrorStatus> {
    sysmem_shm_open(shm_key, flags).get()
}
