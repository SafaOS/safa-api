use core::num::NonZero;

use safa_abi::{
    errors::ErrorStatus,
    ffi::{
        num::ShouldNotBeZero,
        option::{COption, OptZero},
        ptr::FFINonNull,
        slice::Slice,
        str::Str,
    },
    process::{ProcessStdio, RawContextPriority, RawPSpawnConfig, SpawnFlags},
};

use crate::{
    exported_func,
    process::stdio::{systry_get_stderr, systry_get_stdin, systry_get_stdout},
    syscalls::types::{OptionalPtrMut, Pid, RequiredPtr, RequiredPtrMut, Ri, SyscallResult},
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
    SyscallNum::SysPTryCleanUp => {
      /// Attempts to cleanup the process with pid `pid` and returns it's exit status on success
      ///
      /// # Returns
      /// - [`ErrorStatus::InvalidPid`] if the target process doesn't exist at the time of attempted cleanup
      ///
      /// - [`ErrorStatus::Generic`] if the target process isn't dead and awaiting cleanup
      sysp_try_cleanup(pid: Pid, dest_exit_code: OptionalPtrMut<usize>)
    },
    SyscallNum::SysPSpawn => {
        sysp_spawn_inner(path: Str, raw_config: RequiredPtr<RawPSpawnConfig>, dest_pid: OptionalPtrMut<Pid>)
    }
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
    let ptr = RequiredPtrMut::new(&mut dest_exit_code).into();
    err_from_u16!(sysp_wait(pid, ptr), dest_exit_code)
}
#[inline]
/// Attempts to cleanup the process with pid `pid` and returns it's exit status on success
///
/// # Returns
/// - Err([`ErrorStatus::InvalidPid`]) if the target process doesn't exist at the time of attempted cleanup
/// - Ok(None) if the target process isn't dead and awaitng cleanup
/// - Ok(Some(exit_code)) if successful
pub fn try_cleanup(pid: Pid) -> Result<Option<usize>, ErrorStatus> {
    let mut dest_exit_code = 0;
    let ptr = RequiredPtrMut::new(&mut dest_exit_code).into();
    let results = err_from_u16!(sysp_try_cleanup(pid, ptr), dest_exit_code);

    match results {
        Ok(results) => Ok(Some(results)),
        Err(ErrorStatus::Generic) => Ok(None),
        Err(e) => Err(e),
    }
}

exported_func! {
    // doesn't use define_syscall because we use a different signature then the rest of the syscalls
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
    extern "C" fn sysp_spawn(
        name: OptZero<Str>,
        path: Str,
        args: OptZero<Slice<Str>>,
        // flags and return
        flags: SpawnFlags,
        priority: RawContextPriority,
        // stdio
        stdin: COption<Ri>,
        stdout: COption<Ri>,
        stderr: COption<Ri>,
        custom_stack_size: OptZero<ShouldNotBeZero<usize>>,
        dest_pid: OptionalPtrMut<Pid>,
    ) -> SyscallResult {
        let (stdin, stdout, stderr): (Option<_>, Option<_>, Option<_>) =
            (stdin.into(), stdout.into(), stderr.into());

        let stdio = {
            if stdin.is_none() && stdout.is_none() && stderr.is_none() {
                None
            } else {
                let stdout = stdout.or(systry_get_stdout().into());
                let stdin = stdin.or(systry_get_stdin().into());
                let stderr = stderr.or(systry_get_stderr().into());

                Some(ProcessStdio::new(stdout, stdin, stderr))
            }
        };

        let stdio = stdio.as_ref();
        let stdio_ptr = stdio.map(|m| unsafe {FFINonNull::new_unchecked(m as *const _ as *mut _)}).into();

        let (_, mut env) = unsafe { crate::process::env::duplicate_env() };

        let env = unsafe {OptZero::some(Slice::from_raw_parts(env.as_mut_ptr(), env.len()))};
        let config = RawPSpawnConfig::new_from_raw(name, args, env, flags, stdio_ptr, priority, custom_stack_size);

        let raw_config_ptr = unsafe {RequiredPtr::new_unchecked(&config as *const _ as *mut _) };
        sysp_spawn_inner(path, raw_config_ptr, dest_pid)
    }
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
    args: *mut [&str],
    flags: SpawnFlags,
    priority: RawContextPriority,
    stdin: Option<Ri>,
    stdout: Option<Ri>,
    stderr: Option<Ri>,
    custom_stack_size: Option<NonZero<usize>>,
) -> Result<Pid, ErrorStatus> {
    let mut pid = 0;
    let pid_ptr = RequiredPtrMut::new(&mut pid).into();

    let name = name.map(|s| Str::from_str(s)).into();
    let path = Str::from_str(path);
    let args = unsafe { OptZero::some(Slice::from_str_slices_mut(args as *mut [*mut str])) };

    err_from_u16!(
        sysp_spawn(
            name,
            path,
            args,
            flags,
            priority.into(),
            stdin.into(),
            stdout.into(),
            stderr.into(),
            match custom_stack_size {
                None => OptZero::none(),
                Some(size) => OptZero::some(unsafe { ShouldNotBeZero::new_unchecked(size.get()) }),
            },
            pid_ptr,
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
    priority: RawContextPriority,
    stdin: Option<Ri>,
    stdout: Option<Ri>,
    stderr: Option<Ri>,
    custom_stack_size: Option<NonZero<usize>>,
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
            custom_stack_size,
        )
    }
}
