//! Particualry a copy of stdlib's sync.
pub mod condvar;
pub mod futex;
pub mod locks;
pub mod mutex;
pub mod once;
pub use condvar::*;
pub use futex::*;
pub use mutex::*;
pub mod lazy;
pub use lazy::*;
pub use once::*;
