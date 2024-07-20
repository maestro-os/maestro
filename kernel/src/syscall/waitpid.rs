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

//! The `waitpid` system call allows to wait for an event from a child process.

use crate::{
	process::{
		mem_space::copy::SyscallPtr, pid::Pid, regs::Regs, rusage::RUsage, scheduler, Process,
		State,
	},
	syscall::{waitpid::scheduler::SCHEDULER, Args},
};
use core::{ffi::c_int, iter};
use utils::{
	errno,
	errno::{EResult, Errno},
	interrupt::cli,
	lock::IntMutex,
};

/// Wait flag. Returns immediately if no child has exited.
pub const WNOHANG: i32 = 1;
/// Wait flag. Returns if a child has stopped.
pub const WUNTRACED: i32 = 2;
/// Wait flag. Returns if a child has terminated.
pub const WEXITED: i32 = 4;
/// Wait flag. Returns if a stopped child has been resumed by delivery of
/// SIGCONT.
pub const WCONTINUED: i32 = 8;
/// Wait flag. If set, the system call doesn't clear the waitable status of the
/// child.
pub const WNOWAIT: i32 = 0x1000000;

/// Returns an iterator over the IDs of the processes to be watched according to the given
/// constraint.
///
/// Arguments:
/// - `curr_proc` is the current process.
/// - `pid` is the constraint given to the system call.
fn iter_targets(curr_proc: &Process, pid: i32) -> impl Iterator<Item = Pid> + '_ {
	let mut i = 0;
	iter::from_fn(move || {
		let res = match pid {
			..-1 => curr_proc.get_group_processes().get(i).cloned(),
			-1 => curr_proc.get_children().get(i).cloned(),
			0 => curr_proc.get_group_processes().get(i).cloned(),
			_ => (i == 0).then_some(pid as _),
		};
		i += 1;
		res
	})
}

/// Returns the wait status for the given process.
fn get_wstatus(proc: &Process) -> i32 {
	let status = proc.get_exit_status().unwrap_or(0);
	let termsig = proc.get_termsig();
	let wstatus = match proc.get_state() {
		State::Running | State::Sleeping => 0xffff,
		State::Stopped => ((termsig as i32 & 0xff) << 8) | 0x7f,
		State::Zombie => ((status as i32 & 0xff) << 8) | (termsig as i32 & 0x7f),
	};
	// TODO
	/*if coredump {
		wstatus |= 0x80;
	}*/
	wstatus
}

/// Waits upon a process and returns it. If no process can be waited upon, the function returns
/// `None`.
///
/// Arguments:
/// - `curr_proc` is the current process.
/// - `pid` is the constraint given to the system call.
/// - `wstatus` is the pointer to the wait status.
/// - `options` is a set of flags.
/// - `rusage` is the pointer to the resource usage structure.
fn get_waitable(
	curr_proc: &mut Process,
	pid: i32,
	wstatus: &SyscallPtr<i32>,
	options: i32,
	rusage: &SyscallPtr<RUsage>,
) -> EResult<Option<Pid>> {
	let mut empty = true;
	let mut sched = SCHEDULER.get().lock();
	// Find a waitable process
	let proc = iter_targets(curr_proc, pid)
		.inspect(|_| empty = false)
		.filter_map(|pid| sched.get_by_pid(pid))
		// Select a waitable process
		.find(|proc| {
			let proc = proc.lock();
			let state = proc.get_state();
			let stopped = options & WUNTRACED != 0 && matches!(state, State::Stopped);
			let exited = options & WEXITED != 0 && matches!(state, State::Zombie);
			let continued =
				options & WCONTINUED != 0 && matches!(state, State::Running | State::Sleeping);
			proc.is_waitable() && (stopped || exited || continued)
		});
	let Some(proc) = proc else {
		return if empty {
			// No target
			Err(errno!(ECHILD))
		} else {
			Ok(None)
		};
	};
	let mut proc = proc.lock();
	let pid = proc.get_pid();
	// Write values back
	wstatus.copy_to_user(get_wstatus(&proc))?;
	rusage.copy_to_user(proc.get_rusage().clone())?;
	// Clear the waitable flag if requested
	if options & WNOWAIT == 0 {
		proc.clear_waitable();
		// If the process was a zombie, remove it
		if matches!(proc.get_state(), State::Zombie) {
			drop(proc);
			curr_proc.remove_child(pid);
			sched.remove_process(pid);
		}
	}
	Ok(Some(pid))
}

/// Executes the `waitpid` system call.
pub fn do_waitpid(
	pid: i32,
	wstatus: SyscallPtr<i32>,
	options: i32,
	rusage: SyscallPtr<RUsage>,
	regs: &Regs,
) -> EResult<usize> {
	// Sleep until a target process is waitable
	loop {
		super::util::handle_signal(regs);
		cli();
		{
			let proc_mutex = Process::current();
			let mut proc = proc_mutex.lock();
			let result = get_waitable(&mut proc, pid, &wstatus, options, &rusage)?;
			// On success, return
			if let Some(p) = result {
				return Ok(p as _);
			}
			// If the flag is set, do not wait
			if options & WNOHANG != 0 {
				return Ok(0);
			}
			// When a child process is paused or resumed by a signal or is terminated, it
			// changes the state of the current process to wake it up
			proc.set_state(State::Sleeping);
		}
		scheduler::end_tick();
	}
}

pub fn waitpid(
	Args((pid, wstatus, options)): Args<(c_int, SyscallPtr<c_int>, c_int)>,
	regs: &Regs,
) -> EResult<usize> {
	do_waitpid(pid, wstatus, options | WEXITED, SyscallPtr(None), regs)
}
