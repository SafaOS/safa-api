use core::time::Duration;

use safa_abi::{
    errors::ErrorStatus,
    raw::{
        processes::{ContextPriority, TSpawnConfig},
        Optional,
    },
};

use crate::{
    exported_func,
    syscalls::types::{Cid, OptionalPtr, OptionalPtrMut, RequiredPtr, SyscallResult},
};

use super::{define_syscall, SyscallNum};

define_syscall! {
    SyscallNum::SysTExit => {
        /// Exits the current thread, threads don't have an exit code
        /// however if the thread was the last thread in the process,
        /// then the process will exit with code [`code`]
        syst_exit(code: usize) unreachable
    },
    SyscallNum::SysTWait => {
        /// Waits for a child thread with the cid `cid` to exit
        ///
        /// # Returns
        /// - [`ErrorStatus::InvalidTid`] if thread doesn't exist at the time of wait
        syst_wait(cid: Cid)
    },
    SyscallNum::SysTSleep => {
      /// Sleeps for N ms
      ///
      /// should always succeed
      syst_sleep(n: usize)
    },
    SyscallNum::SysTYield => {
        /// Switches to the next thread in the thread queue of the current CPU
        sysyield()
    },
}

/// Exits the current thread, threads don't have an exit code
/// however if the thread was the last thread in the process,
/// then the process will exit with code `code`
#[inline]
pub fn exit(code: usize) -> ! {
    syst_exit(code)
}

/// Switches to the next thread in the thread queue of the current CPU
#[inline]
pub fn yield_now() {
    debug_assert!(sysyield().is_success())
}

#[inline]
/// Waits for the thread with the id `cid` to exit
//
/// # Returns
///
/// - [`ErrorStatus::InvalidTid`] if the target thread doesn't exist at the time of wait
pub fn wait(cid: Cid) -> Result<(), ErrorStatus> {
    err_from_u16!(syst_wait(cid))
}

#[inline]
/// Sleeps for a given duration
///
/// # Returns
/// - Err(()) if duration as milliseconds is larger than usize::MAX
pub fn sleep(duration: Duration) -> Result<(), ()> {
    let ms: usize = duration.as_millis().try_into().map_err(|_| ())?;
    Ok(assert!(syst_sleep(ms).is_success()))
}

define_syscall! {
    SyscallNum::SysTSpawn => {
        /// Spawns a thread at the entry point `entry_point` with the config `config`
        ///
        /// - if `dest_cid` is not null it will be set to the spawned thread's ID (CID or TID)
        syst_spawn_raw(entry_point: usize, config: RequiredPtr<TSpawnConfig>, dest_cid: OptionalPtrMut<Cid>)
    },
}

exported_func! {
    /// Spawns a thread as a child of self
    /// # Arguments
    /// - `entry_point`: a pointer to the main function of the thread,
    /// the main function looks like this: `fn main(thread_id: Cid, argument_ptr: usize)` see `dest_cid` below for thread_id, argument_ptr == `argument_ptr`
    ///
    /// - `argument_ptr`: a pointer to the arguments that will be passed to the thread, this pointer will be based as is,
    /// and therefore can also be used to pass a single usize argument
    ///
    /// - `priotrity`: the pritority of the thread in the thread queue, will default to the parent's
    ///
    /// - `dest_cid`: if not null, will be set to the thread ID of the spawned thread
    extern "C" fn syst_spawn(
        entry_point: usize,
        argument_ptr: OptionalPtr<()>,
        priority: Optional<ContextPriority>,
        custom_stack_size: Optional<usize>,
        dest_cid: OptionalPtrMut<Cid>,
    ) -> SyscallResult {
        let config = TSpawnConfig::new(
            argument_ptr,
            priority.into(),
            None,
            custom_stack_size.into(),
        );
        syscall!(
            SyscallNum::SysTSpawn,
            entry_point,
            &config as *const _ as usize,
            dest_cid as usize
        )
    }
}

#[inline]
/// Spawns a thread as a child of self
/// unlike [`spawn`], this will pass no arguments to the thread
/// # Arguments
/// - `entry_point`: a pointer to the main function of the thread
///
/// - `priotrity`: the pritority of the thread in the thread queue, will default to the parent's
///
/// # Returns
/// - the thread ID of the spawned thread
pub fn spawn3(
    entry_point: fn(thread_id: Cid) -> !,
    priority: Option<ContextPriority>,
    custom_stack_size: Option<usize>,
) -> Result<Cid, ErrorStatus> {
    spawn_inner(
        entry_point as usize,
        core::ptr::null(),
        priority,
        custom_stack_size,
    )
}

#[inline]
/// Spawns a thread as a child of self
/// unlike [`spawn`], instead of taking a reference as an argument to pass to the thread, this will take a usize
/// # Arguments
/// - `entry_point`: a pointer to the main function of the thread
///
/// - `argument`: a usize argument that will be passed to the thread
///
/// - `priotrity`: the pritority of the thread in the thread queue, will default to the parent's
///
/// # Returns
/// - the thread ID of the spawned thread
pub fn spawn2(
    entry_point: fn(thread_id: Cid, argument: usize) -> !,
    argument: usize,
    priority: Option<ContextPriority>,
    custom_stack_size: Option<usize>,
) -> Result<Cid, ErrorStatus> {
    spawn_inner(
        entry_point as usize,
        argument as *const (),
        priority,
        custom_stack_size,
    )
}

#[inline]
/// Spawns a thread as a child of self
/// # Arguments
/// - `entry_point`: a pointer to the main function of the thread
///
/// - `argument_ptr`: a pointer to the arguments that will be passed to the thread, this pointer will be based as is,
///
/// - `priotrity`: the pritority of the thread in the thread queue, will default to the parent's
///
/// # Returns
/// - the thread ID of the spawned thread
pub fn spawn<T>(
    entry_point: fn(thread_id: Cid, argument_ptr: &'static T) -> !,
    argument_ptr: &'static T,
    priority: Option<ContextPriority>,
    custom_stack_size: Option<usize>,
) -> Result<Cid, ErrorStatus> {
    spawn_inner(
        entry_point as usize,
        argument_ptr as *const T as *const (),
        priority,
        custom_stack_size,
    )
}

#[inline(always)]
fn spawn_inner(
    entry_point: usize,
    argument_ptr: *const (),
    priority: Option<ContextPriority>,
    custom_stack_size: Option<usize>,
) -> Result<Cid, ErrorStatus> {
    let mut cid = 0;
    err_from_u16!(
        syst_spawn(
            entry_point,
            argument_ptr,
            priority.into(),
            custom_stack_size.into(),
            &mut cid
        ),
        cid
    )
}
