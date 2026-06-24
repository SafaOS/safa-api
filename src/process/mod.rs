//! Module for process-related high-level functions over process related syscalls
//!
//! Such as api initialization functions [`init::_c_api_init`] and [`init::sysapi_init`], environment variables, and process arguments

use core::{cell::UnsafeCell, mem::MaybeUninit};

use safa_abi::process::AbiStructures;

pub mod args;
pub mod env;
#[cfg(not(feature = "std"))]
pub mod init;
pub mod stdio;
pub use init::*;

struct StaticAbiStructures(UnsafeCell<MaybeUninit<AbiStructures>>);

impl StaticAbiStructures {
    pub unsafe fn init(&self, structures: AbiStructures) {
        let ptr = self.0.get();
        ptr.write(MaybeUninit::new(structures));
    }

    pub unsafe fn get(&'static self) -> &'static AbiStructures {
        let ptr = self.0.get();
        MaybeUninit::assume_init_ref(&*ptr)
    }
}

unsafe impl Sync for StaticAbiStructures {}

#[cfg_attr(feature = "linkonce", unsafe(no_mangle))]
#[cfg_attr(feature = "linkonce", linkage = "weak")]
static SAAPI_ABI_STRUCTURES: StaticAbiStructures =
    StaticAbiStructures(UnsafeCell::new(MaybeUninit::zeroed()));

/// Returns the current [`AbiStructures`].
/// Must be run after init().
pub fn proc_meta() -> &'static AbiStructures {
    unsafe { SAAPI_ABI_STRUCTURES.get() }
}

#[derive(Debug, Clone, Copy)]
/// Information given about the running executable (now for use with the dynamic linker/program interpreter) at boot.
pub struct ElfExeInfo {
    /// Program Entry Point (not Interpreter's) (available all the time).
    pub at_entry: *const (),
    /// Programs header address (not Interpreter's) available when a PT_PHDR header is.
    pub at_phdr: *const (),
    /// Size of a program header entry (available when a PT_PHDR header is).
    pub at_phent: usize,
    /// Count of program headers (available when a PT_PHDR header is).
    pub at_phnum: usize,

    /// Program Interpreter Base.
    pub at_base: *const (),
}

impl ElfExeInfo {
    pub fn get() -> Self {
        Self {
            at_base: proc_meta().at_base as *const (),
            at_entry: proc_meta().at_entry as *const (),
            at_phdr: proc_meta().at_phdr as *const (),
            at_phent: proc_meta().at_phent,
            at_phnum: proc_meta().at_phnum,
        }
    }
}

/// Sets the [`AbiStructures`].
pub(self) unsafe fn init_proc_meta(value: AbiStructures) {
    unsafe { SAAPI_ABI_STRUCTURES.init(value) }
}
