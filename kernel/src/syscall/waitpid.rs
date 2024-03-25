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
	process,
	process::{
		mem_space::ptr::SyscallPtr, pid::Pid, regs::Regs, rusage::RUsage, scheduler, Process,
		State,
	},
};
use core::ffi::c_int;
use macros::syscall;
use utils::{
	errno,
	errno::{EResult, Errno},
	interrupt::cli,
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

/// Returns the `i`th target process for the given constraint `pid`.
///
/// Arguments:
/// - `curr_proc` is the current process.
/// - `pid` is the constraint given to the system call.
/// - `i` is the index of the target process.
///
/// The function is built such as iterating on `i` until the function returns
/// `None` gives every targets for the system call.
fn get_target(curr_proc: &Process, pid: i32, i: usize) -> Option<Pid> {
	if pid < -1 {
		let group_processes = curr_proc.get_group_processes();

		if i < group_processes.len() {
			Some(group_processes[i])
		} else {
			None
		}
	} else if pid == -1 {
		let children = curr_proc.get_children();

		if i < children.len() {
			Some(children[i])
		} else {
			None
		}
	} else if pid == 0 {
		let group = curr_proc.get_group_processes();

		if i < group.len() {
			Some(group[i])
		} else {
			None
		}
	} else if i == 0 {
		Some(pid as _)
	} else {
		None
	}
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

/// Checks if at least one process corresponding to the given constraint is
/// waitable. If yes, the function clears its waitable state, sets the wstatus
/// and returns the process's PID.
///
/// Arguments:
/// - `curr_proc` is the current process.
/// - `pid` is the constraint given to the system call.
/// - `wstatus` is a reference to the wait status.
/// - `options` is a set of flags.
/// - `rusage` is the pointer to the resource usage structure.
fn check_waitable(
	curr_proc: &mut Process,
	pid: i32,
	wstatus: &mut i32,
	options: i32,
	rusage: &mut RUsage,
) -> EResult<Option<Pid>> {
	// Iterating on every target processes, checking if they can be waited on
	let mut i = 0;
	while let Some(pid) = get_target(curr_proc, pid, i) {
		let mut sched = process::get_scheduler().lock();

		if let Some(p) = sched.get_by_pid(pid) {
			let mut p = p.lock();

			let stopped = matches!(p.get_state(), State::Stopped);
			let zombie = matches!(p.get_state(), State::Zombie);
			let running = matches!(p.get_state(), State::Running | State::Sleeping);

			let stop_check = stopped && options & WUNTRACED != 0;
			let exit_check = zombie && options & WEXITED != 0;
			let continue_check = running && options & WCONTINUED != 0;

			// If waitable, return
			if p.is_waitable() && (stop_check || exit_check || continue_check) {
				*wstatus = get_wstatus(&p);
				*rusage = p.get_rusage().clone();

				let clear_waitable = options & WNOWAIT == 0;
				if clear_waitable {
					p.clear_waitable();

					// If the process was a zombie, remove it
					if exit_check {
						drop(p);

						curr_proc.remove_child(pid);
						sched.remove_process(pid);
					}
				}

				return Ok(Some(pid));
			}
		}

		i += 1;
	}

	if i == 0 {
		// No target
		Err(errno!(ECHILD))
	} else {
		Ok(None)
	}
}

/// Executes the `waitpid` system call.
///
/// Arguments:
/// - `regs` is the registers state.
/// - `pid` is the PID to wait for.
/// - `wstatus` is the pointer on which to write the status.
/// - `options` are flags passed with the syscall.
/// - `rusage` is the pointer to the resource usage structure.
pub fn do_waitpid(
	regs: &Regs,
	pid: i32,
	wstatus: SyscallPtr<i32>,
	options: i32,
	rusage: Option<SyscallPtr<RUsage>>,
) -> EResult<i32> {
	// Sleeping until a target process is waitable
	loop {
		super::util::handle_signal(regs);

		cli();

		{
			let proc_mutex = Process::current_assert();
			let mut proc = proc_mutex.lock();

			// Check if at least one target process is waitable
			let mut wstatus_val = Default::default();
			let mut rusage_val = Default::default();
			let result =
				check_waitable(&mut proc, pid, &mut wstatus_val, options, &mut rusage_val)?;

			// Setting values to userspace
			{
				let mem_space = proc.get_mem_space().unwrap();
				let mut mem_space_guard = mem_space.lock();

				if let Some(wstatus) = wstatus.get_mut(&mut mem_space_guard)? {
					*wstatus = wstatus_val;
				}

				if let Some(ref rusage) = rusage {
					if let Some(rusage) = rusage.get_mut(&mut mem_space_guard)? {
						*rusage = rusage_val;
					}
				}
			}

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

#[syscall]
pub fn waitpid(pid: c_int, wstatus: SyscallPtr<c_int>, options: c_int) -> Result<i32, Errno> {
	do_waitpid(regs, pid, wstatus, options | WEXITED, None)
}
