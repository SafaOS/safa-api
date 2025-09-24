use safa_abi::errors::ErrorStatus;

use crate::syscalls::types::{RequiredPtrMut, Ri};

use super::{define_syscall, err_from_u16, SyscallNum};
define_syscall! {
    SyscallNum::SysRDestroy => {
        /// Destroys "closes" a resource with the id `ri`, a resource can be a File, a Directory, a DirIter, etc...
        ///
        /// # Returns
        /// - [`ErrorStatus::InvalidResource`] if the id `ri` is invalid
        sysr_destroy(ri: Ri)
    },
    SyscallNum::SysRDup => {
        /// Duplicates the resource referred to by the resource id `ri` and puts the new resource id in `dest_ri`
        sysr_dup(ri: Ri, dest_ri: RequiredPtrMut<Ri>)
    }
}

/// Destroys "closes" a resource with the id `ri`, a resource can be a File, Directory, DirIter, etc...
///
/// # Returns
/// - [`ErrorStatus::InvalidResource`] if the id `ri` is invalid
#[inline]
pub fn destroy_resource(ri: Ri) -> Result<(), ErrorStatus> {
    err_from_u16!(sysr_destroy(ri))
}

#[inline]
/// Duplicates the resource referred to by the resource id `ri`
/// and returns the new resource id
pub fn dup(ri: Ri) -> Result<Ri, ErrorStatus> {
    let mut dest_ri = 0xAAAAAAAAAAAAAAAAusize as Ri;
    let ptr = unsafe { RequiredPtrMut::new_unchecked(&mut dest_ri) };
    err_from_u16!(sysr_dup(ri, ptr), dest_ri)
}
