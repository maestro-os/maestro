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

//! This module implements the `kill` system call, which allows to send a signal
//! to a process.

use super::util;
use crate::{
	process,
	process::{pid::Pid, signal::Signal, Process, State},
};
use core::ffi::c_int;
use macros::syscall;
use utils::{
	errno,
	errno::{EResult, Errno},
	interrupt::cli,
};

/// Tries to kill the process with PID `pid` with the signal `sig`.
///
/// If `sig` is `None`, the function doesn't send a signal, but still checks if
/// there is a process that could be killed.
fn try_kill(pid: Pid, sig: &Option<Signal>) -> EResult<()> {
	let proc_mutex = Process::current_assert();
	let mut proc = proc_mutex.lock();
	let ap = proc.access_profile;
	// Closure sending the signal
	let f = |target: &mut Process| {
		if matches!(target.get_state(), State::Zombie) {
			return Err(errno!(ESRCH));
		}
		if !ap.can_kill(target) {
			return Err(errno!(EPERM));
		}
		if let Some(sig) = sig {
			target.kill(sig);
		}
		Ok(())
	};
	if pid == proc.pid {
		f(&mut proc)?;
	} else {
		let target_mutex = Process::get_by_pid(pid).ok_or_else(|| errno!(ESRCH))?;
		let mut target_proc = target_mutex.lock();
		f(&mut target_proc)?;
	}
	Ok(())
}

/// Tries to kill a process group.
///
/// Arguments:
/// - `pid` is the value that determine which process(es) to kill.
/// - `sig` is the signal to send.
///
/// If `sig` is `None`, the function doesn't send a signal, but still checks if
/// there is a process that could be killed.
fn try_kill_group(pid: i32, sig: &Option<Signal>) -> EResult<()> {
	let pgid = match pid {
		0 => {
			let proc_mutex = Process::current_assert();
			let proc = proc_mutex.lock();
			proc.pgid
		}
		i if i < 0 => -pid as Pid,
		_ => pid as Pid,
	};
	// Kill process group
	{
		let proc_mutex = Process::get_by_pid(pgid).ok_or_else(|| errno!(ESRCH))?;
		let proc = proc_mutex.lock();
		let group = proc.get_group_processes();
		for pid in group {
			if *pid == pgid {
				continue;
			}
			try_kill(*pid as _, sig)?;
		}
	}
	// Kill process group owner
	try_kill(pgid, sig)?;
	Ok(())
}

/// Sends the signal `sig` to the processes according to the given value `pid`.
/// If `sig` is `None`, the function doesn't send a signal, but still checks if
/// there is a process that could be killed.
fn send_signal(pid: i32, sig: Option<Signal>) -> EResult<()> {
	if pid > 0 {
		// Kill the process with the given PID
		try_kill(pid as _, &sig)
	} else if pid == 0 {
		// Kill all processes in the current process group
		try_kill_group(0, &sig)
	} else if pid == -1 {
		// Kill all processes for which the current process has the permission
		let mut sched = process::get_scheduler().lock();
		for (pid, _) in sched.iter_process() {
			if *pid == process::pid::INIT_PID {
				continue;
			}
			// TODO Check permission
			try_kill(*pid, &sig)?;
		}
		Ok(())
	} else if pid < -1 {
		// Kill the given process group
		try_kill_group(-pid as _, &sig)
	} else {
		Err(errno!(ESRCH))
	}
}

#[syscall]
pub fn kill(pid: c_int, sig: c_int) -> Result<i32, Errno> {
	// Validation
	if sig < 0 {
		return Err(errno!(EINVAL));
	}
	let sig = if sig > 0 {
		Some(Signal::try_from(sig as u32)?)
	} else {
		None
	};
	// TODO check if necessary
	cli();
	send_signal(pid, sig)?;
	// Setting the return value of the system call so that it is correct even if a signal is
	// executed before returning
	{
		let proc_mutex = Process::current_assert();
		let mut proc = proc_mutex.lock();
		let mut return_regs = proc.regs.clone();
		return_regs.set_syscall_return(Ok(0));
		proc.regs = return_regs;
	}
	// If the current process has been killed, the system call must execute the signal before
	// returning FIXME: this must be done only if no other thread has the signal unblocked or
	// listening to the signal
	util::handle_signal(regs);
	Ok(0)
}
