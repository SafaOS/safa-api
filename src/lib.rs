#![no_std]
pub mod errors {
    pub use safa_utils::errors::{ErrorStatus, SysResult};
}

pub mod syscalls;
