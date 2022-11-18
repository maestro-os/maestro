//! The `prlimit64` syscall returns the limit for a given resource.

use crate::errno::Errno;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::process::pid::Pid;
use crate::process::Process;
use core::ffi::c_int;
use macros::syscall;

/// The amount of seconds of CPU time the process can consume.
const RLIMIT_CPU: i32 = 0;
/// The maximum size of a file the process may create, in bytes.
const RLIMIT_FSIZE: i32 = 1;
/// The maximum size of the process's data segment in bytes, rounded down to the
/// page size.
const RLIMIT_DATA: i32 = 2;
/// TODO doc
const RLIMIT_STACK: i32 = 3;
/// The maximum size of a core file the process may dump in bytes.
const RLIMIT_CORE: i32 = 4;
/// TODO doc
const RLIMIT_RSS: i32 = 5;
/// TODO doc
const RLIMIT_NPROC: i32 = 6;
/// A value one greater than the maximum number of file descriptors that can be
/// open by the process.
const RLIMIT_NOFILE: i32 = 7;
/// TODO doc
const RLIMIT_MEMLOCK: i32 = 8;
/// The maximum size of the memory space in bytes, rounded down to the page
/// size.
const RLIMIT_AS: i32 = 9;
/// The limit on the combined number of flock(2) locks and fcntl(2) leases the
/// process may establish.
const RLIMIT_LOCKS: i32 = 10;
/// TODO doc
const RLIMIT_SIGPENDING: i32 = 11;
/// TODO doc
const RLIMIT_MSGQUEUE: i32 = 12;
/// TODO doc
const RLIMIT_NICE: i32 = 13;
/// TODO doc
const RLIMIT_RTPRIO: i32 = 14;
/// TODO doc
const RLIMIT_RTTIME: i32 = 15;
/// TODO doc
const RLIMIT_NLIMITS: i32 = 16;

/// Type representing a resource limit.
type RLim = u64;

/// Structure representing a resource limit.
#[repr(C)]
struct RLimit {
	/// Soft limit
	rlim_cur: RLim,
	/// Hard limit (ceiling for rlim_cur)
	rlim_max: RLim,
}

// TODO Check args types
/// The implementation of the `prlimit64` syscall.
#[syscall]
pub fn prlimit64(
	pid: Pid,
	resource: c_int,
	_new_limit: SyscallPtr<RLimit>,
	_old_limit: SyscallPtr<RLimit>,
) -> Result<i32, Errno> {
	// The target process. If None, the current process is the target
	let _target_proc = if pid == 0 {
		None
	} else {
		// TODO Check permission
		Some(Process::get_by_pid(pid).ok_or_else(|| errno!(ESRCH))?)
	};

	// The current process
	let mutex = Process::get_current().unwrap();
	let guard = mutex.lock();
	let proc = guard.get_mut();

	let mem_space_mutex = proc.get_mem_space().unwrap();
	let _mem_space_guard = mem_space_mutex.lock();

	// TODO Implement all
	match resource {
		RLIMIT_CPU => {}
		RLIMIT_FSIZE => {}
		RLIMIT_DATA => {}
		RLIMIT_STACK => {}
		RLIMIT_CORE => {}
		RLIMIT_RSS => {}
		RLIMIT_NPROC => {}
		RLIMIT_NOFILE => {}
		RLIMIT_MEMLOCK => {}
		RLIMIT_AS => {}
		RLIMIT_LOCKS => {}
		RLIMIT_SIGPENDING => {}
		RLIMIT_MSGQUEUE => {}
		RLIMIT_NICE => {}
		RLIMIT_RTPRIO => {}
		RLIMIT_RTTIME => {}
		RLIMIT_NLIMITS => {}

		_ => return Err(errno!(EINVAL)),
	}

	Ok(0)
}
