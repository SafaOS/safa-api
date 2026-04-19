//! SafaOS VTTYs Implementation
//!
//! VTTYs can work as a pipe.
use crate::{
    errors::ErrorStatus,
    resource::Resource,
    syscalls::{self, types::Ri},
};

#[derive(Debug)]
pub struct MotherVTTY {
    resource: Resource,
}

#[derive(Debug)]
pub struct ChildVTTY {
    resource: Resource,
}

impl MotherVTTY {
    pub const SET_FLAGS: u16 = 1;

    #[inline(always)]
    pub const fn ri(&self) -> Ri {
        self.resource.ri()
    }

    #[inline(always)]
    pub const fn resource(&self) -> &Resource {
        &self.resource
    }

    /// Reads data from the VTTY at the specified offset into the provided buffer.
    pub fn read(&self, offset: isize, buf: &mut [u8]) -> Result<usize, ErrorStatus> {
        unsafe { self.resource.read(offset, buf) }
    }

    /// Sends a command to the VTTY with the specified command and argument.
    pub fn send_command(&self, command: u16, argument: u64) -> Result<(), ErrorStatus> {
        unsafe { self.resource.io_command(command, argument) }
    }

    /// Sets the flags for the VTTY.
    pub fn set_flags(&self, flags: u64) -> Result<(), ErrorStatus> {
        self.send_command(Self::SET_FLAGS, flags)
    }
}

impl ChildVTTY {
    #[inline(always)]
    pub const fn resource(&self) -> &Resource {
        &self.resource
    }

    /// Writes data to the VTTY at the specified offset from the provided buffer.
    pub fn write(&self, offset: isize, buf: &[u8]) -> Result<usize, ErrorStatus> {
        unsafe { self.resource.write(offset, buf) }
    }

    /// Reads data from the VTTY at the specified offset into the provided buffer.
    pub fn read(&self, offset: isize, buf: &mut [u8]) -> Result<usize, ErrorStatus> {
        unsafe { self.resource.read(offset, buf) }
    }
}

/// Construct new pair of (`MotherVTTY`, `ChildVTTY`)
pub fn new() -> (MotherVTTY, ChildVTTY) {
    let (mother_ri, child_ri) = syscalls::io::vtty_alloc().expect("Failed to allocate VTTY");
    unsafe {
        (
            MotherVTTY {
                resource: Resource::from_raw(mother_ri),
            },
            ChildVTTY {
                resource: Resource::from_raw(child_ri),
            },
        )
    }
}
