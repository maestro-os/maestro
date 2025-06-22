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

#[cfg(target_arch = "x86_64")]
use crate::{arch::x86, syscall::FromSyscallArg};
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
	hint::unlikely,
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

/// Set 64 bit base for the FS register.
const ARCH_SET_GS: c_int = 0x1001;
/// Set 64 bit base for the GS register.
const ARCH_SET_FS: c_int = 0x1002;
/// Get 64 bit base value for the FS register.
const ARCH_GET_FS: c_int = 0x1003;
/// Get 64 bit base value for the GS register.
const ARCH_GET_GS: c_int = 0x1004;
/// Tells whether the cpuid instruction is enabled.
const ARCH_GET_CPUID: c_int = 0x1011;
/// Enable or disable cpuid instruction.
const ARCH_SET_CPUID: c_int = 0x1012;

/// Returns the resource usage of the current process.
const RUSAGE_SELF: i32 = 0;
/// Returns the resource usage of the process's children.
const RUSAGE_CHILDREN: i32 = -1;

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
/// A limit on the process's resident set (the number of virtual pages resident in RAM).
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
/// The limit on the number of bytes that can be allocated for POSIX message queues for the real
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
		SCHEDULER.swap_current_process(child.clone());
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
fn get_tls_entry(
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
	let (id, entry) = get_tls_entry(&mut entries, info.get_entry_number())?;
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

#[allow(unused_variables)]
pub fn arch_prctl(Args((code, addr)): Args<(c_int, usize)>) -> EResult<usize> {
	// For `gs`, use kernel base because it will get swapped when returning to userspace
	match code {
		#[cfg(target_arch = "x86_64")]
		ARCH_SET_GS => x86::wrmsr(x86::IA32_KERNEL_GS_BASE, addr as _),
		#[cfg(target_arch = "x86_64")]
		ARCH_SET_FS => x86::wrmsr(x86::IA32_FS_BASE, addr as _),
		#[cfg(target_arch = "x86_64")]
		ARCH_GET_FS => {
			let val = x86::rdmsr(x86::IA32_FS_BASE) as usize;
			let ptr = UserPtr::<usize>::from_ptr(addr);
			ptr.copy_to_user(&val)?;
		}
		#[cfg(target_arch = "x86_64")]
		ARCH_GET_GS => {
			let val = x86::rdmsr(x86::IA32_GS_BASE) as usize;
			let ptr = UserPtr::<usize>::from_ptr(addr);
			ptr.copy_to_user(&val)?;
		}
		// TODO ARCH_GET_CPUID
		// TODO ARCH_SET_CPUID
		_ => return Err(errno!(EINVAL)),
	}
	#[allow(unreachable_code)]
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

/// A resource limit.
#[repr(C)]
#[derive(Debug)]
pub struct RLimit {
	/// Soft limit
	rlim_cur: u64,
	/// Hard limit (ceiling for [`rlim_cur`])
	rlim_max: u64,
}

pub fn prlimit64(
	Args((pid, resource, _new_limit, _old_limit)): Args<(
		Pid,
		c_int,
		UserPtr<RLimit>,
		UserPtr<RLimit>,
	)>,
) -> EResult<usize> {
	// The target process. If None, the current process is the target
	let _target_proc = if pid != 0 {
		// TODO Check permission
		Some(Process::get_by_pid(pid).ok_or_else(|| errno!(ESRCH))?)
	} else {
		None
	};
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

pub fn sched_yield() -> EResult<usize> {
	Scheduler::tick();
	Ok(0)
}

/// Exits the current process.
///
/// Arguments:
/// - `status` is the exit status.
/// - `thread_group`: if `true`, the function exits the whole process group.
/// - `proc` is the current process.
pub fn do_exit(status: u32, thread_group: bool) -> ! {
	// Disable interruptions to prevent execution from being stopped before the reference to
	// `Process` is dropped
	cli();
	{
		let proc = Process::current();
		proc.exit(status);
		let _pid = proc.get_pid();
		let _tid = proc.tid;
		if thread_group {
			// TODO Iterate on every process of thread group `tid`, except the
			// process with pid `pid`
		}
	}
	Scheduler::tick();
	// Cannot resume since the process is now a zombie
	unreachable!();
}

pub fn _exit(Args(status): Args<c_int>) -> EResult<usize> {
	do_exit(status as _, false);
}

pub fn exit_group(Args(status): Args<c_int>) -> EResult<usize> {
	do_exit(status as _, true);
}
