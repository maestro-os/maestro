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

//! The `kill` system call, which allows to send a signal to a process.

use super::{util, Args};
use crate::{
	process,
	process::{pid::Pid, scheduler::SCHEDULER, signal::Signal, Process, State},
};
use core::ffi::c_int;
use utils::{
	errno,
	errno::{EResult, Errno},
};

/// Tries to kill the process with PID `pid` with the signal `sig`.
///
/// If `sig` is `None`, the function doesn't send a signal, but still checks if
/// there is a process that could be killed.
fn try_kill(pid: Pid, sig: Option<Signal>) -> EResult<()> {
	let proc = Process::current();
	let ap = proc.fs.lock().access_profile;
	// Closure sending the signal
	let f = |target: &Process| {
		if matches!(target.get_state(), State::Zombie) {
			return Ok(());
		}
		if !ap.can_kill(target) {
			return Err(errno!(EPERM));
		}
		if let Some(sig) = sig {
			target.kill(sig);
		}
		Ok(())
	};
	if pid == proc.get_pid() {
		f(&proc)?;
	} else {
		let target_proc = Process::get_by_pid(pid).ok_or_else(|| errno!(ESRCH))?;
		f(&target_proc)?;
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
fn try_kill_group(pid: i32, sig: Option<Signal>) -> EResult<()> {
	let pgid = match pid {
		0 => Process::current().get_pgid(),
		i if i < 0 => -pid as Pid,
		_ => pid as Pid,
	};
	// Kill process group
	Process::get_by_pid(pgid)
		.ok_or_else(|| errno!(ESRCH))?
		.links
		.lock()
		.process_group
		.iter()
		.try_for_each(|pid| try_kill(*pid as _, sig))
}

pub fn kill(Args((pid, sig)): Args<(c_int, c_int)>) -> EResult<usize> {
	let sig = (sig != 0).then(|| Signal::try_from(sig)).transpose()?;
	match pid {
		// Kill the process with the given PID
		1.. => try_kill(pid as _, sig)?,
		// Kill all processes in the current process group
		0 => try_kill_group(0, sig)?,
		// Kill all processes for which the current process has the permission
		-1 => {
			let sched = SCHEDULER.lock();
			for (pid, _) in sched.iter_process() {
				if *pid == process::pid::INIT_PID {
					continue;
				}
				// TODO Check permission
				try_kill(*pid, sig)?;
			}
		}
		// Kill the given process group
		..-1 => try_kill_group(-pid as _, sig)?,
	}
	Ok(0)
}
