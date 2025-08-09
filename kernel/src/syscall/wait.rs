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
	memory::user::UserPtr,
	process::{Process, State, pid::Pid, rusage::Rusage, scheduler::schedule},
	syscall::Args,
};
use core::{
	ffi::c_int,
	iter,
	sync::atomic::Ordering::{Acquire, Release},
};
use utils::{errno, errno::EResult};

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
		// FIXME: select only process that are children of `curr_proc`
		let links = curr_proc.links.lock();
		let res = match pid {
			// FIXME: must wait for any child process whose pgid is equal to -pid
			..-1 => links.process_group.get(i).cloned(),
			-1 => links.children.get(i).cloned(),
			0 => links.process_group.get(i).cloned(),
			_ => (i == 0).then_some(pid as _),
		};
		i += 1;
		res
	})
}

/// Returns the wait status for the given process.
fn get_wstatus(proc: &Process) -> i32 {
	let (status, termsig) = {
		let signal = proc.signal.lock();
		(signal.exit_status, signal.termsig)
	};
	#[allow(clippy::let_and_return)]
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
	curr_proc: &Process,
	pid: i32,
	wstatus: UserPtr<i32>,
	options: i32,
	rusage: UserPtr<Rusage>,
) -> EResult<Option<Pid>> {
	let mut empty = true;
	// Find a waitable process
	let proc = iter_targets(curr_proc, pid)
		.inspect(|_| empty = false)
		.filter_map(Process::get_by_pid)
		// Select a waitable process
		.find(|proc| {
			let events = if options & WNOWAIT == 0 {
				proc.parent_event.fetch_and(!(options as u8), Release)
			} else {
				proc.parent_event.load(Acquire)
			};
			let stopped = options & WUNTRACED != 0 && events & WUNTRACED as u8 != 0;
			let exited = options & WEXITED != 0 && proc.get_state() == State::Zombie;
			let continued = options & WCONTINUED != 0 && events & WCONTINUED as u8 != 0;
			stopped || exited || continued
		});
	let Some(proc) = proc else {
		return if empty {
			// No target
			Err(errno!(ECHILD))
		} else {
			Ok(None)
		};
	};
	// Write values back
	wstatus.copy_to_user(&get_wstatus(&proc))?;
	rusage.copy_to_user(&proc.rusage.lock())?;
	// Remove zombie process if requested
	let pid = proc.get_pid();
	if options & WNOWAIT == 0 && proc.get_state() == State::Zombie {
		Process::remove(proc);
	}
	Ok(Some(pid))
}

/// Executes the `waitpid` system call.
pub fn do_waitpid(
	pid: i32,
	wstatus: UserPtr<i32>,
	options: i32,
	rusage: UserPtr<Rusage>,
) -> EResult<usize> {
	loop {
		{
			let proc = Process::current();
			let result = get_waitable(&proc, pid, wstatus, options, rusage.clone())?;
			// On success, return
			if let Some(p) = result {
				return Ok(p as _);
			}
			// If the flag is set, do not wait
			if options & WNOHANG != 0 {
				return Ok(0);
			}
			// When a child process has its state changed by a signal, SIGCHLD is sent to the
			// current process to wake it up
			Process::set_state(&proc, State::Sleeping);
		}
		schedule();
	}
}

#[allow(missing_docs)]
pub fn waitpid(
	Args((pid, wstatus, options)): Args<(c_int, UserPtr<c_int>, c_int)>,
) -> EResult<usize> {
	do_waitpid(pid, wstatus, options | WEXITED, UserPtr(None))
}

#[allow(missing_docs)]
pub fn wait4(
	Args((pid, wstatus, options, rusage)): Args<(c_int, UserPtr<c_int>, c_int, UserPtr<Rusage>)>,
) -> EResult<usize> {
	do_waitpid(pid, wstatus, options | WEXITED, rusage)
}
