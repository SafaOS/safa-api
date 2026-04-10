use safa_abi::{errors::ErrorStatus, fs::OpenOptions};

use crate::syscalls::{self, types::Ri};

/// Represents an open resource.
///
/// [`Self::destroy`] is called on drop.
#[derive(Debug)]
pub struct Resource(Ri);

impl Resource {
    /// Safety: `raw` must be a valid resource.
    #[inline]
    pub unsafe fn from_raw(raw: Ri) -> Self {
        Self(raw)
    }

    #[inline]
    pub const fn ri(&self) -> Ri {
        self.0
    }

    #[inline]
    /// [`syscalls::fs::open`].
    pub fn open(path: &str, options: OpenOptions) -> Result<Self, ErrorStatus> {
        syscalls::fs::open(path, options).map(|ri| Resource(ri))
    }

    #[inline]
    /// [`syscalls::io::read`].
    pub unsafe fn read(&self, offset: isize, buf: &mut [u8]) -> Result<usize, ErrorStatus> {
        syscalls::io::read(self.0, offset, buf)
    }

    #[inline]
    /// [`syscalls::io::write`].
    pub unsafe fn write(&self, offset: isize, buf: &[u8]) -> Result<usize, ErrorStatus> {
        syscalls::io::write(self.0, offset, buf)
    }

    #[inline]
    /// [`syscalls::resources::destroy_resource`].
    ///
    /// Allows handling of errors instead of just panicking on drop in case of any.
    ///
    /// No Errors shall be produced on drop in case of a valid produced resource.
    pub fn destroy(self) -> Result<(), ErrorStatus> {
        let ri = self.0;
        core::mem::forget(self);
        syscalls::resources::destroy(ri)
    }

    #[inline]
    /// [`syscalls::io::io_command`].
    pub unsafe fn io_command(&self, cmd: u16, arg: u64) -> Result<(), ErrorStatus> {
        syscalls::io::io_command(self.0, cmd, arg)
    }

    #[inline]
    /// Attempts to create a new resource pointing to the same data.
    pub fn clone(&self) -> Result<Resource, ErrorStatus> {
        syscalls::resources::dup(self.ri()).map(|ri| Resource(ri))
    }
}

impl Drop for Resource {
    fn drop(&mut self) {
        syscalls::resources::destroy(self.0).expect("Failed to drop resource")
    }
}
