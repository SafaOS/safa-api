use safa_abi::errors::ErrorStatus;

use crate::syscalls::types::Ri;

use super::{define_syscall, SyscallNum};
define_syscall! {
    SyscallNum::SysRDestroy => {
        /// Destroys "closes" a resource with the id `ri`, a resource can be a File, a Directory, a DirIter, etc...
        ///
        /// # Returns
        /// - [`ErrorStatus::InvalidResource`] if the id `ri` is invalid
        sysr_destroy(ri: Ri)
    },
    SyscallNum::SysRClone => {
        /// Clones the resource referred to by the resource id `ri` and returns a new resource ID.
        sysr_clone(ri: Ri) Ri
    }
}

/// Destroys "closes" a resource with the id `ri`, a resource can be a File, Directory, DirIter, etc...
///
/// # Returns
/// - [`ErrorStatus::InvalidResource`] if the id `ri` is invalid
#[inline]
pub fn destroy_resource(ri: Ri) -> Result<(), ErrorStatus> {
    sysr_destroy(ri).get()
}

#[inline]
/// Duplicates the resource referred to by the resource id `ri`
/// and returns the new resource id
pub fn dup(ri: Ri) -> Result<Ri, ErrorStatus> {
    sysr_clone(ri).get()
}
