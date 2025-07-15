use safa_abi::{
    errors::ErrorStatus,
    raw::{
        processes::{ContextPriority, SpawnFlags, TaskStdio},
        Optional, RawSlice, RawSliceMut,
    },
};

use crate::{
    process::stdio::{sysmeta_stderr, sysmeta_stdin, sysmeta_stdout},
    syscalls::types::{OptionalPtrMut, OptionalStrPtr, Pid, Ri, StrPtr, SyscallResult},
};

use super::{define_syscall, SyscallNum};

#[cfg(not(feature = "rustc-dep-of-std"))]
extern crate alloc;
use alloc::vec::Vec;

define_syscall! {
    SyscallNum::SysPExit => {
        /// Exits the process with the exit code [`code`]
        sysp_exit(code: usize) unreachable
    },
    SyscallNum::SysPWait => {
        /// Waits for a child process with the pid `pid` to exit
        ///
        /// # Returns
        /// - [`ErrorStatus::InvalidPid`] if the target process doesn't exist at the time of wait
        ///
        /// - [`ErrorStatus::MissingPermissions`] if the target process isn't a child of self
        ///
        /// - if `exit_code` is not null, it will be set to the exit code of the process if successful
        sysp_wait(pid: Pid, exit_code: OptionalPtrMut<usize>)
    },
}

/// Exits the process with the exit code `code`
#[inline]
pub fn exit(code: usize) -> ! {
    sysp_exit(code)
}

#[inline]
/// Waits for the process with the resource id `pid` to exit
/// and returns the exit code of the process
/// # Returns
/// - Ok(exit_code) if the target process was found, was a child of self, and was awaited successfully
///
/// - [`ErrorStatus::InvalidPid`] if the target process doesn't exist at the time of wait
///
/// - [`ErrorStatus::MissingPermissions`] if the target process isn't a child of self
pub fn wait(pid: Pid) -> Result<usize, ErrorStatus> {
    let mut dest_exit_code = 0;
    err_from_u16!(sysp_wait(pid, &mut dest_exit_code), dest_exit_code)
}

// doesn't use define_syscall because we use a different signature then the rest of the syscalls
#[cfg_attr(
    not(any(feature = "std", feature = "rustc-dep-of-std")),
    unsafe(no_mangle)
)]
#[inline(always)]
/// Spawns a new process with the path `path` with arguments `argv` and flags `flags`
/// name_ptr can be null, in which case the name will be the path
/// path and name must be valid utf-8
///
/// - if `dest_pid` is not null, it will be set to the pid of the new process
///
/// - if `stdin`, `stdout`, or `stderr` are not `None`, the corresponding file descriptors will be inherited from the parent
///   if they are None they will be inherited from the parent
///
/// - the behavior isn't defined if `priority` is None, currently it will be set to a default
extern "C" fn syspspawn(
    name_ptr: OptionalStrPtr,
    name_len: usize,
    path_ptr: StrPtr,
    path_len: usize,
    // args
    argv_ptr: OptionalPtrMut<RawSlice<u8>>,
    argv_len: usize,
    // flags and return
    flags: SpawnFlags,
    priority: Optional<ContextPriority>,
    dest_pid: OptionalPtrMut<Pid>,
    // stdio
    stdin: Optional<Ri>,
    stdout: Optional<Ri>,
    stderr: Optional<Ri>,
) -> SyscallResult {
    use safa_abi::raw::processes::PSpawnConfig;
    let (mut stdin, mut stdout, mut stderr): (Option<_>, Option<_>, Option<_>) =
        (stdin.into(), stdout.into(), stderr.into());

    let stdio = {
        if stdin.is_none() && stdout.is_none() && stderr.is_none() {
            None
        } else {
            stdout.get_or_insert_with(|| sysmeta_stdout());
            stdin.get_or_insert_with(|| sysmeta_stdin());
            stderr.get_or_insert_with(|| sysmeta_stderr());

            Some(TaskStdio::new(stdout, stdin, stderr))
        }
    };

    let stdio = stdio.as_ref();
    let stdio_ptr = stdio.map(|m| m as *const _).unwrap_or(core::ptr::null());
    let (_, mut env) = unsafe { crate::process::env::duplicate_env() };

    let config = PSpawnConfig {
        revision: 2,
        name: unsafe { RawSlice::from_raw_parts(name_ptr, name_len) },
        argv: unsafe { RawSliceMut::from_raw_parts(argv_ptr, argv_len) },
        env: unsafe { RawSliceMut::from_raw_parts(env.as_mut_ptr(), env.len()) },
        flags,
        stdio: stdio_ptr,
        priority,
    };

    syscall!(
        SyscallNum::SysPSpawn,
        path_ptr as usize,
        path_len,
        (&raw const config) as usize,
        dest_pid as *mut _ as usize,
    )
}

/// spawns a new process
/// # Arguments
/// * `stdin`, `stdout`, `stderr` are the file descriptors of stdio, if None, they will be inherited from the parent
/// * `priority` is the process's default priority (that the threads, including the root one, will inherit by default),
/// if set to None the behavior isn't well defined, however for now it will default to a constant value
/// # Safety
/// - `argv` must be valid pointers to a slice of slices of `&str`
///
/// - `argv` will become invalid after use, using them is UB
#[inline]
pub unsafe fn unsafe_spawn(
    name: Option<&str>,
    path: &str,
    argv: *mut [&str],
    flags: SpawnFlags,
    priority: Option<ContextPriority>,
    stdin: Option<Ri>,
    stdout: Option<Ri>,
    stderr: Option<Ri>,
) -> Result<Pid, ErrorStatus> {
    let mut pid = 0;

    let name = name.map(|s| s.as_bytes());
    let name_ptr = name.map(|s| s.as_ptr()).unwrap_or(core::ptr::null());
    let name_len = name.map(|s| s.len()).unwrap_or(0);

    let argv = argv as *mut [&[u8]];
    let argv = unsafe { RawSliceMut::from_slices(argv) };

    err_from_u16!(
        syspspawn(
            name_ptr,
            name_len,
            path.as_ptr(),
            path.len(),
            argv.as_mut_ptr(),
            argv.len(),
            flags,
            priority.into(),
            &mut pid,
            stdin.into(),
            stdout.into(),
            stderr.into(),
        ),
        pid
    )
}

// FIXME: I convert the argv form &[u8] to RawSlice
// and that is why it is consumed,
// i reuse the argv buffer as the result of the conversion
// even though this might be inefficient especially that RawSlice should have the same layout as &[u8] and same for env
// although this is fine because this method is only used in the rust standard library which gives args as an owned Vec anyways
//
/// same as [`unsafe_spawn`] but safe because it makes it clear that `argv`  are consumed
#[inline]
pub fn spawn(
    name: Option<&str>,
    path: &str,
    mut argv: Vec<&str>,
    flags: SpawnFlags,
    priority: Option<ContextPriority>,
    stdin: Option<Ri>,
    stdout: Option<Ri>,
    stderr: Option<Ri>,
) -> Result<Pid, ErrorStatus> {
    let argv: &mut [&str] = &mut argv;
    unsafe {
        unsafe_spawn(
            name,
            path,
            argv as *mut _,
            flags,
            priority,
            stdin,
            stdout,
            stderr,
        )
    }
}
