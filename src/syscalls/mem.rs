use core::ptr::NonNull;

use safa_abi::errors::ErrorStatus;
use safa_abi::mem::{MemMapFlags, RawMemMapConfig};

use crate::syscalls::types::{IntoSyscallArg, Ri};

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
