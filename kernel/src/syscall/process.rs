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

//! Process management system calls.

use crate::{
	arch::x86::{cli, gdt, idt::IntFrame},
	memory::user::UserPtr,
	process,
	process::{
		ForkOptions, Process, State,
		pid::Pid,
		rusage::Rusage,
		scheduler::{
			SCHEDULER, Scheduler, switch,
			switch::{fork_asm, stash_segments},
		},
		user_desc::UserDesc,
	},
	syscall::Args,
};
use core::{
	ffi::{c_int, c_ulong, c_void},
	intrinsics::unlikely,
	ptr::null_mut,
};
use utils::{errno, errno::EResult, ptr::arc::Arc};

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

/// The index of the first entry for TLS segments in the GDT.
const TLS_BEGIN_INDEX: usize = gdt::TLS_OFFSET / size_of::<gdt::Entry>();

/// Returns the resource usage of the current process.
const RUSAGE_SELF: i32 = 0;
/// Returns the resource usage of the process's children.
const RUSAGE_CHILDREN: i32 = -1;

pub fn getpid(proc: Arc<Process>) -> EResult<usize> {
	Ok(proc.get_pid() as _)
}

pub fn getppid(proc: Arc<Process>) -> EResult<usize> {
	Ok(proc.get_parent_pid() as _)
}

pub fn getpgid(Args(pid): Args<Pid>) -> EResult<usize> {
	if pid == 0 {
		let proc = Process::current();
		Ok(proc.get_pgid() as _)
	} else {
		let Some(proc) = Process::get_by_pid(pid) else {
			return Err(errno!(ESRCH));
		};
		Ok(proc.get_pgid() as _)
	}
}

pub fn setpgid(Args((mut pid, mut pgid)): Args<(Pid, Pid)>, proc: Arc<Process>) -> EResult<usize> {
	// TODO Check processes SID
	if pid == 0 {
		pid = proc.get_pid();
	}
	if pgid == 0 {
		pgid = pid;
	}
	if pid == proc.get_pid() {
		proc.set_pgid(pgid)?;
	} else {
		// Avoid deadlock
		drop(proc);
		Process::get_by_pid(pid)
			.ok_or_else(|| errno!(ESRCH))?
			.set_pgid(pgid)?;
	}
	Ok(0)
}

pub fn gettid(proc: Arc<Process>) -> EResult<usize> {
	Ok(proc.tid as _)
}

pub fn set_tid_address(Args(_tidptr): Args<UserPtr<c_int>>, proc: Arc<Process>) -> EResult<usize> {
	// TODO set process's clear_child_tid
	Ok(proc.tid as _)
}

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
		UserPtr<c_int>,
		c_ulong,
		UserPtr<c_int>,
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
		SCHEDULER.lock().swap_current_process(child.clone());
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
		UserPtr<c_int>,
		UserPtr<c_int>,
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

pub fn fork(proc: Arc<Process>, frame: &mut IntFrame) -> EResult<usize> {
	clone(
		Args((0, null_mut(), UserPtr(None), UserPtr(None), 0)),
		proc,
		frame,
	)
}

pub fn vfork(proc: Arc<Process>, frame: &mut IntFrame) -> EResult<usize> {
	clone(
		Args((
			CLONE_VFORK | CLONE_VM,
			null_mut(),
			UserPtr(None),
			UserPtr(None),
			0,
		)),
		proc,
		frame,
	)
}

/// Returns an entry ID for the given process and entry number.
///
/// If the id is `-1`, the function shall find a free entry.
fn get_entry(
	entries: &mut [gdt::Entry; process::TLS_ENTRIES_COUNT],
	entry_number: i32,
) -> EResult<(usize, &mut gdt::Entry)> {
	const BEGIN_ENTRY: i32 = TLS_BEGIN_INDEX as i32;
	const END_ENTRY: i32 = BEGIN_ENTRY + process::TLS_ENTRIES_COUNT as i32;
	let id = match entry_number {
		// Find a free entry
		-1 => entries
			.iter()
			.enumerate()
			.find(|(_, e)| !e.is_present())
			.map(|(i, _)| i)
			.ok_or(errno!(ESRCH))?,
		// Valid entry index
		BEGIN_ENTRY..END_ENTRY => (entry_number - BEGIN_ENTRY) as usize,
		// Out of bounds
		_ => return Err(errno!(EINVAL)),
	};
	Ok((id, &mut entries[id]))
}

pub fn set_thread_area(
	Args(u_info): Args<UserPtr<UserDesc>>,
	proc: Arc<Process>,
) -> EResult<usize> {
	// Read user_desc
	let mut info = u_info.copy_from_user()?.ok_or(errno!(EFAULT))?;
	// Get the entry with its id
	let mut entries = proc.tls.lock();
	let (id, entry) = get_entry(&mut entries, info.get_entry_number())?;
	// If the entry is allocated, tell the userspace its ID
	let entry_number = info.get_entry_number();
	if entry_number == -1 {
		info.set_entry_number((TLS_BEGIN_INDEX + id) as _);
		u_info.copy_to_user(&info)?;
	}
	// Update the entry
	*entry = info.to_descriptor();
	unsafe {
		entry.update_gdt(gdt::TLS_OFFSET + id * size_of::<gdt::Entry>());
	}
	gdt::flush();
	Ok(0)
}

pub fn getrusage(Args((who, usage)): Args<(c_int, UserPtr<Rusage>)>) -> EResult<usize> {
	let proc = Process::current();
	let rusage = match who {
		RUSAGE_SELF => proc.rusage.lock().clone(),
		RUSAGE_CHILDREN => {
			// TODO Return resources of terminated children
			Rusage::default()
		}
		_ => return Err(errno!(EINVAL)),
	};
	usage.copy_to_user(&rusage)?;
	Ok(0)
}

pub fn sched_yield() -> EResult<usize> {
	Scheduler::tick();
	Ok(0)
}
