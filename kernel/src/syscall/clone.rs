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

use crate::process::{
	mem_space::ptr::SyscallPtr, scheduler, user_desc::UserDesc, ForkOptions, Process,
};
use core::ffi::c_void;
use macros::syscall;
use utils::{errno::Errno, ptr::arc::Arc};

/// TODO doc
const CLONE_IO: i32 = -0x80000000;
/// If specified, the parent and child processes share the same memory space.
const CLONE_VM: i32 = 0x100;
/// TODO doc
const CLONE_FS: i32 = 0x200;
/// If specified, the parent and child processes share the same file descriptors
/// table.
const CLONE_FILES: i32 = 0x400;
/// If specified, the parent and child processes share the same signal handlers
/// table.
const CLONE_SIGHAND: i32 = 0x800;
/// TODO doc
const CLONE_PIDFD: i32 = 0x1000;
/// TODO doc
const CLONE_PTRACE: i32 = 0x2000;
/// TODO doc
const CLONE_VFORK: i32 = 0x4000;
/// TODO doc
const CLONE_PARENT: i32 = 0x8000;
/// TODO doc
const CLONE_THREAD: i32 = 0x10000;
/// TODO doc
const CLONE_NEWNS: i32 = 0x20000;
/// TODO doc
const CLONE_SYSVSEM: i32 = 0x40000;
/// TODO doc
const CLONE_SETTLS: i32 = 0x80000;
/// TODO doc
const CLONE_PARENT_SETTID: i32 = 0x100000;
/// TODO doc
const CLONE_CHILD_CLEARTID: i32 = 0x200000;
/// TODO doc
const CLONE_DETACHED: i32 = 0x400000;
/// TODO doc
const CLONE_UNTRACED: i32 = 0x800000;
/// TODO doc
const CLONE_CHILD_SETTID: i32 = 0x1000000;
/// TODO doc
const CLONE_NEWCGROUP: i32 = 0x2000000;
/// TODO doc
const CLONE_NEWUTS: i32 = 0x4000000;
/// TODO doc
const CLONE_NEWIPC: i32 = 0x8000000;
/// TODO doc
const CLONE_NEWUSER: i32 = 0x10000000;
/// TODO doc
const CLONE_NEWPID: i32 = 0x20000000;
/// TODO doc
const CLONE_NEWNET: i32 = 0x40000000;

// TODO Check args types
#[syscall]
pub fn clone(
	flags: i32,
	stack: *mut c_void,
	_parent_tid: SyscallPtr<i32>,
	tls: i32,
	_child_tid: SyscallPtr<i32>,
) -> Result<i32, Errno> {
	let new_tid = {
		// The current process
		let curr_mutex = Process::current_assert();
		// A weak pointer to the new process's parent
		let parent = Arc::downgrade(&curr_mutex);

		let mut curr_proc = curr_mutex.lock();

		if flags & CLONE_PARENT_SETTID != 0 {
			// TODO
			todo!();
		}

		let fork_options = ForkOptions {
			share_memory: flags & CLONE_VM != 0,
			share_fd: flags & CLONE_FILES != 0,
			share_sighand: flags & CLONE_SIGHAND != 0,

			vfork: flags & CLONE_VFORK != 0,
		};
		let new_mutex = curr_proc.fork(parent, fork_options)?;
		let mut new_proc = new_mutex.lock();

		// Setting the process's registers
		let mut new_regs = regs.clone();
		// Setting return value to `0`
		new_regs.eax = 0;
		// Setting stack
		new_regs.esp = if stack.is_null() {
			regs.esp as _
		} else {
			stack as _
		};
		// Setting TLS
		if flags & CLONE_SETTLS != 0 {
			let _tls: SyscallPtr<UserDesc> = (tls as usize).into();

			// TODO
			todo!();
		}
		new_proc.regs = new_regs;

		if flags & CLONE_CHILD_CLEARTID != 0 {
			// TODO new_proc.set_clear_child_tid(child_tid);
			todo!();
		}
		if flags & CLONE_CHILD_SETTID != 0 {
			// TODO
			todo!();
		}

		new_proc.tid
	};

	if flags & CLONE_VFORK != 0 {
		// Letting another process run instead of the current. Because the current
		// process must now wait for the child process to terminate or execute a program
		scheduler::end_tick();
	}

	Ok(new_tid as _)
}
