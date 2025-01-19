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
	arch::x86::{cli, idt::IntFrame},
	process::{
		mem_space::copy::SyscallPtr,
		pid::Pid,
		scheduler,
		scheduler::{
			switch,
			switch::{fork_asm, init_ctx, stash_segments},
			Scheduler, SCHEDULER,
		},
		user_desc::UserDesc,
		ForkOptions, Process, State,
	},
	syscall::{Args, FromSyscallArg},
};
use core::{
	ffi::{c_int, c_ulong, c_void},
	intrinsics::unlikely,
	ptr::NonNull,
	sync::atomic::Ordering::Relaxed,
};
use utils::{errno::EResult, ptr::arc::Arc};

/// TODO doc
pub const CLONE_IO: c_ulong = -0x80000000 as _;
/// If specified, the parent and child processes share the same memory space.
pub const CLONE_VM: c_ulong = 0x100;
/// TODO doc
pub const CLONE_FS: c_ulong = 0x200;
/// If specified, the parent and child processes share the same file descriptors
/// table.
pub const CLONE_FILES: c_ulong = 0x400;
/// If specified, the parent and child processes share the same signal handlers
/// table.
pub const CLONE_SIGHAND: c_ulong = 0x800;
/// TODO doc
pub const CLONE_PIDFD: c_ulong = 0x1000;
/// TODO doc
pub const CLONE_PTRACE: c_ulong = 0x2000;
/// TODO doc
pub const CLONE_VFORK: c_ulong = 0x4000;
/// TODO doc
pub const CLONE_PARENT: c_ulong = 0x8000;
/// TODO doc
pub const CLONE_THREAD: c_ulong = 0x10000;
/// TODO doc
pub const CLONE_NEWNS: c_ulong = 0x20000;
/// TODO doc
pub const CLONE_SYSVSEM: c_ulong = 0x40000;
/// TODO doc
pub const CLONE_SETTLS: c_ulong = 0x80000;
/// TODO doc
pub const CLONE_PARENT_SETTID: c_ulong = 0x100000;
/// TODO doc
pub const CLONE_CHILD_CLEARTID: c_ulong = 0x200000;
/// TODO doc
pub const CLONE_DETACHED: c_ulong = 0x400000;
/// TODO doc
pub const CLONE_UNTRACED: c_ulong = 0x800000;
/// TODO doc
pub const CLONE_CHILD_SETTID: c_ulong = 0x1000000;
/// TODO doc
pub const CLONE_NEWCGROUP: c_ulong = 0x2000000;
/// TODO doc
pub const CLONE_NEWUTS: c_ulong = 0x4000000;
/// TODO doc
pub const CLONE_NEWIPC: c_ulong = 0x8000000;
/// TODO doc
pub const CLONE_NEWUSER: c_ulong = 0x10000000;
/// TODO doc
pub const CLONE_NEWPID: c_ulong = 0x20000000;
/// TODO doc
pub const CLONE_NEWNET: c_ulong = 0x40000000;

/// Wait for the vfork operation to complete.
fn wait_vfork_done(child_pid: Pid) {
	loop {
		// Use a scope to avoid holding references that could be lost, since `tick` could never
		// return
		{
			let proc = Process::current();
			let Some(child) = Process::get_by_pid(child_pid) else {
				// Child disappeared for some reason, stop
				break;
			};
			// If done, stop waiting
			if child.is_vfork_done() {
				break;
			}
			// Sleep until done
			proc.set_state(State::Sleeping);
			// If vfork has completed in between, cancel sleeping
			if unlikely(child.is_vfork_done()) {
				proc.set_state(State::Running);
				break;
			}
		}
		// Let another process run while we wait
		Scheduler::tick();
	}
}

#[allow(clippy::type_complexity)]
pub fn compat_clone(
	Args((flags, stack, _parent_tid, _tls, _child_tid)): Args<(
		c_ulong,
		*mut c_void,
		SyscallPtr<c_int>,
		c_ulong,
		SyscallPtr<c_int>,
	)>,
	proc: Arc<Process>,
	frame: &mut IntFrame,
) -> EResult<usize> {
	let (child_pid, child_tid) = {
		// Disable interruptions so that the scheduler does not attempt to start the new process
		cli();
		let child = Process::fork(
			proc.clone(),
			ForkOptions {
				share_memory: flags & CLONE_VM != 0,
				share_fd: flags & CLONE_FILES != 0,
				share_sighand: flags & CLONE_SIGHAND != 0,
			},
		)?;
		let child_pid = child.get_pid();
		let child_tid = child.tid;
		// Switch
		switch::finish(&proc, &child);
		SCHEDULER.get().lock().swap_current_process(child.clone());
		let mut child_frame = frame.clone();
		child_frame.rax = 0; // Return value
		if !stack.is_null() {
			child_frame.rsp = stack as _;
		}
		stash_segments(|| unsafe {
			fork_asm(Arc::as_ptr(&proc), Arc::as_ptr(&child), &child_frame);
		});
		(child_pid, child_tid)
	};
	if flags & CLONE_VFORK != 0 {
		wait_vfork_done(child_pid);
	}
	Ok(child_tid as _)
}

#[allow(clippy::type_complexity)]
pub fn clone(
	Args((flags, stack, parent_tid, child_tid, tls)): Args<(
		c_ulong,
		*mut c_void,
		SyscallPtr<c_int>,
		SyscallPtr<c_int>,
		c_ulong,
	)>,
	proc: Arc<Process>,
	frame: &mut IntFrame,
) -> EResult<usize> {
	compat_clone(
		Args((flags, stack, parent_tid, tls, child_tid)),
		proc,
		frame,
	)
}
