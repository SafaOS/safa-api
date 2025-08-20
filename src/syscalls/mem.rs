use core::ptr::NonNull;

use safa_abi::errors::ErrorStatus;
use safa_abi::mem::{MemMapFlags, RawMemMapConfig, ShmFlags};

use crate::syscalls::types::{IntoSyscallArg, RequiredPtrMut, Ri};

use super::types::{OptionalPtrMut, RequiredPtr};
use super::SyscallNum;

impl IntoSyscallArg for MemMapFlags {
    type RegResults = (usize,);
    fn into_syscall_arg(self) -> Self::RegResults {
        (unsafe { core::mem::transmute::<_, u8>(self) } as usize,)
    }
}

define_syscall! {
    SyscallNum::SysMemMap => {
        /// See [`SyscallNum::SysMemMap`]
        sysmem_map(memmap_config: RequiredPtr<RawMemMapConfig>, flags: MemMapFlags, out_res_id: OptionalPtrMut<Ri>, out_start_addr: OptionalPtrMut<NonNull<u8>>)
    }
}

/// See [`SyscallNum::SysMemMap`] and [`RawMemMapConfig`]
///
/// You don't have to provide the [`MemMapFlags::MAP_RESOURCE`] flag, it is automatically provided if `resource_to_map` is Some
/// # Returns
/// the resource ID of the Tracked Mapping and the slice of bytes in the mapping
pub fn map(
    addr_hint: *const (),
    page_count: usize,
    guard_pages_count: usize,
    resource_to_map: Option<Ri>,
    resource_off: Option<isize>,
    mut flags: MemMapFlags,
) -> Result<(Ri, NonNull<[u8]>), ErrorStatus> {
    let (ri, off) = if let Some(ri) = resource_to_map {
        let off = resource_off.unwrap_or_default();
        flags = flags | MemMapFlags::MAP_RESOURCE;
        (ri, off)
    } else {
        (0, 0)
    };

    let conf = RawMemMapConfig {
        resource_off: off,
        resource_to_map: ri,
        guard_pages_count,
        page_count,
        addr_hint,
    };

    let mut res_id_results = 0xAAAAAAAAAAAAAAAAusize;
    let mut start_addr_results =
        unsafe { NonNull::new_unchecked(0xAAAAAAAAAAAAAAAAusize as *mut u8) };
    let (result_ri, result_start_addr) = unsafe {
        err_from_u16!(
            sysmem_map(
                RequiredPtr::new_unchecked(&raw const conf as *mut _),
                flags,
                RequiredPtr::new(&raw mut res_id_results).into(),
                RequiredPtr::new(&raw mut start_addr_results).into()
            ),
            (res_id_results, start_addr_results)
        )?
    };

    // each Page is 4096 bytes
    let len = page_count * 4096;
    let slice = unsafe { core::slice::from_raw_parts_mut(result_start_addr.as_ptr(), len) };

    unsafe { Ok((result_ri, NonNull::new_unchecked(slice))) }
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
        /// * `out_resource_id` - An optional pointer to a [`Ri`] that will be filled with the resource ID of the created shared memory descriptor, as if a call to [`sysmem_shm_open`] was made.
        sysmem_shm_create(page_count: usize, flags: ShmFlags, out_shm_key: RequiredPtrMut<ShmKey>, out_resource_id: OptionalPtrMut<Ri>)
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
        /// * `out_resource_id` - A pointer to a [`Ri`] that will be filled with the resource ID of the opened shared memory descriptor.
        sysmem_shm_open(shm_key: ShmKey, flags: ShmFlags, out_resource_id: RequiredPtrMut<Ri>)
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
    let mut resource_id = Ri::default();
    unsafe {
        err_from_u16!(
            sysmem_shm_create(
                page_count,
                flags,
                RequiredPtrMut::new_unchecked(&mut shm_key),
                RequiredPtrMut::new(&mut resource_id).into()
            ),
            (shm_key, resource_id)
        )
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
    let mut resource_id = Ri::default();
    unsafe {
        err_from_u16!(
            sysmem_shm_open(
                shm_key,
                flags,
                RequiredPtrMut::new_unchecked(&mut resource_id)
            ),
            resource_id
        )
    }
}
