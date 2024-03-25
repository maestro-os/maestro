/*
 * Copyright 2024 Luc Lenôtre
 *
 * This file is part of Maestro.
 *
 * Maestro is free software: you can redistribute it and/or modify it under the
 * terms of the GNU General Public License as published by the Free Software
 * Foundation, either version 3 of the License, or (at your option) any later
 * version.
 *
 * Maestro is distributed in the hope that it will be useful, but WITHOUT ANY
 * WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR
 * A PARTICULAR PURPOSE. See the GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along with
 * Maestro. If not, see <https://www.gnu.org/licenses/>.
 */

//! The `prlimit64` syscall returns the limit for a given resource.

use crate::process::{mem_space::ptr::SyscallPtr, pid::Pid, Process};
use core::ffi::c_int;
use macros::syscall;
use utils::{errno, errno::Errno};

/// The amount of seconds of CPU time the process can consume.
const RLIMIT_CPU: i32 = 0;
/// The maximum size of a file the process may create, in bytes.
const RLIMIT_FSIZE: i32 = 1;
/// The maximum size of the process's data segment in bytes, rounded down to the
/// page size.
const RLIMIT_DATA: i32 = 2;
/// The maximum size of the process stack, in bytes.
const RLIMIT_STACK: i32 = 3;
/// The maximum size of a kernel file the process may dump in bytes.
const RLIMIT_CORE: i32 = 4;
/// A limit on the process's resident set (the numbe rof virtual pages resident in RAM).
const RLIMIT_RSS: i32 = 5;
/// The limit on the number of threads for the real user ID of the calling process.
const RLIMIT_NPROC: i32 = 6;
/// A value one greater than the maximum number of file descriptors that can be
/// open by the process.
const RLIMIT_NOFILE: i32 = 7;
/// The maximum number of butes of memory that may be locked into RAM.
const RLIMIT_MEMLOCK: i32 = 8;
/// The maximum size of the memory space in bytes, rounded down to the page
/// size.
const RLIMIT_AS: i32 = 9;
/// The limit on the combined number of flock(2) locks and fcntl(2) leases the
/// process may establish.
const RLIMIT_LOCKS: i32 = 10;
/// The limit on the number of signals that may be queued for the real user ID of the calling
/// process.
const RLIMIT_SIGPENDING: i32 = 11;
/// The limit on the number of butes that can be allocated for POSIX message queues for the real
/// user IF of the calling process.
const RLIMIT_MSGQUEUE: i32 = 12;
/// The ceiling to which the process's nice value can be raised.
const RLIMIT_NICE: i32 = 13;
/// The ceiling on the real-time priority that may be set for this process.
const RLIMIT_RTPRIO: i32 = 14;
/// The limit (in microseconds) on the amount of CPU that a process scheduled under a real-time
/// scheduling policy may consume without masking a blocking system call.
const RLIMIT_RTTIME: i32 = 15;
/// TODO doc
const RLIMIT_NLIMITS: i32 = 16;

/// Type representing a resource limit.
type RLim = u64;

/// Structure representing a resource limit.
#[repr(C)]
#[derive(Debug)]
struct RLimit {
	/// Soft limit
	rlim_cur: RLim,
	/// Hard limit (ceiling for rlim_cur)
	rlim_max: RLim,
}

// TODO Check args types
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

	let proc_mutex = Process::current_assert();
	let proc = proc_mutex.lock();

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
