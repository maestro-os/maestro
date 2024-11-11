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

//! The `clone` system call creates a child process.

use crate::{
	arch::x86::idt::IntFrame,
	process::{
		mem_space::copy::SyscallPtr, scheduler, scheduler::Scheduler, user_desc::UserDesc,
		ForkOptions, Process,
	},
	syscall::{Args, FromSyscallArg},
};
use core::{
	ffi::{c_int, c_ulong, c_void},
	ptr::NonNull,
};
use utils::{errno::EResult, lock::IntMutex, ptr::arc::Arc};

/// TODO doc
const CLONE_IO: c_ulong = -0x80000000 as _;
/// If specified, the parent and child processes share the same memory space.
const CLONE_VM: c_ulong = 0x100;
/// TODO doc
const CLONE_FS: c_ulong = 0x200;
/// If specified, the parent and child processes share the same file descriptors
/// table.
const CLONE_FILES: c_ulong = 0x400;
/// If specified, the parent and child processes share the same signal handlers
/// table.
const CLONE_SIGHAND: c_ulong = 0x800;
/// TODO doc
const CLONE_PIDFD: c_ulong = 0x1000;
/// TODO doc
const CLONE_PTRACE: c_ulong = 0x2000;
/// TODO doc
const CLONE_VFORK: c_ulong = 0x4000;
/// TODO doc
const CLONE_PARENT: c_ulong = 0x8000;
/// TODO doc
const CLONE_THREAD: c_ulong = 0x10000;
/// TODO doc
const CLONE_NEWNS: c_ulong = 0x20000;
/// TODO doc
const CLONE_SYSVSEM: c_ulong = 0x40000;
/// TODO doc
const CLONE_SETTLS: c_ulong = 0x80000;
/// TODO doc
const CLONE_PARENT_SETTID: c_ulong = 0x100000;
/// TODO doc
const CLONE_CHILD_CLEARTID: c_ulong = 0x200000;
/// TODO doc
const CLONE_DETACHED: c_ulong = 0x400000;
/// TODO doc
const CLONE_UNTRACED: c_ulong = 0x800000;
/// TODO doc
const CLONE_CHILD_SETTID: c_ulong = 0x1000000;
/// TODO doc
const CLONE_NEWCGROUP: c_ulong = 0x2000000;
/// TODO doc
const CLONE_NEWUTS: c_ulong = 0x4000000;
/// TODO doc
const CLONE_NEWIPC: c_ulong = 0x8000000;
/// TODO doc
const CLONE_NEWUSER: c_ulong = 0x10000000;
/// TODO doc
const CLONE_NEWPID: c_ulong = 0x20000000;
/// TODO doc
const CLONE_NEWNET: c_ulong = 0x40000000;

#[allow(clippy::type_complexity)]
pub fn clone(
	Args((flags, stack, _parent_tid, _tls, _child_tid)): Args<(
		c_ulong,
		*mut c_void,
		SyscallPtr<c_int>,
		c_ulong,
		SyscallPtr<c_int>,
	)>,
	proc: Arc<Process>,
) -> EResult<usize> {
	let new_tid = {
		if flags & CLONE_PARENT_SETTID != 0 {
			todo!()
		}
		let new_proc = Process::fork(
			proc,
			ForkOptions {
				share_memory: flags & CLONE_VM != 0,
				share_fd: flags & CLONE_FILES != 0,
				share_sighand: flags & CLONE_SIGHAND != 0,

				vfork: flags & CLONE_VFORK != 0,

				stack: NonNull::new(stack),
			},
		)?;
		if flags & CLONE_SETTLS != 0 {
			todo!()
		}
		if flags & CLONE_CHILD_CLEARTID != 0 {
			todo!()
		}
		if flags & CLONE_CHILD_SETTID != 0 {
			todo!()
		}
		new_proc.tid
	};
	if flags & CLONE_VFORK != 0 {
		// Let another process run instead of the current. Because the current
		// process must now wait for the child process to terminate or execute a program
		Scheduler::tick();
	}
	Ok(new_tid as _)
}
