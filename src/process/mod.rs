//! Module for process-related high-level functions over process related syscalls
//!
//! Such as api initialization functions [`init::_c_api_init`] and [`init::sysapi_init`], environment variables, and process arguments

pub mod args;
pub mod env;
#[cfg(not(feature = "std"))]
pub mod init;
pub mod stdio;
