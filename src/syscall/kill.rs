//! This module implements the `kill` system call, which allows to send a signal
//! to a process.

use super::util;
use crate::errno;
use crate::errno::Errno;
use crate::process;
use crate::process::pid::Pid;
use crate::process::signal::Signal;
use crate::process::Process;
use crate::process::State;
use core::ffi::c_int;
use macros::syscall;

/// Tries to kill the process with PID `pid` with the signal `sig`.
///
/// If `sig` is `None`, the function doesn't send a signal, but still checks if
/// there is a process that could be killed.
fn try_kill(pid: Pid, sig: &Option<Signal>) -> Result<(), Errno> {
	let proc_mutex = Process::current_assert();
	let mut curr_proc = proc_mutex.lock();

	let uid = curr_proc.uid;
	let euid = curr_proc.euid;

	// Closure sending the signal
	let f = |target: &mut Process| {
		if matches!(target.get_state(), State::Zombie) {
			return Err(errno!(ESRCH));
		}
		if !target.can_kill(uid) && !target.can_kill(euid) {
			return Err(errno!(EPERM));
		}

		if let Some(sig) = sig {
			target.kill(sig, false);
		}

		Ok(())
	};

	if pid == curr_proc.pid {
		f(&mut curr_proc)?;
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
/// - `pid` is the a value that determine which process(es) to kill.
/// - `sig` is the signal to send.
///
/// If `sig` is `None`, the function doesn't send a signal, but still checks if
/// there is a process that could be killed.
fn try_kill_group(pid: i32, sig: &Option<Signal>) -> Result<(), Errno> {
	let pgid = match pid {
		0 => {
			let curr_mutex = Process::current_assert();
			let curr_proc = curr_mutex.lock();

			curr_proc.pgid
		}

		i if i < 0 => -pid as Pid,
		_ => pid as Pid,
	};

	// Killing process group
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

	// Killing process group owner
	try_kill(pgid, sig)?;

	Ok(())
}

/// Sends the signal `sig` to the processes according to the given value `pid`.
/// If `sig` is `None`, the function doesn't send a signal, but still checks if
/// there is a process that could be killed.
fn send_signal(pid: i32, sig: Option<Signal>) -> Result<(), Errno> {
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
	if sig < 0 {
		return Err(errno!(EINVAL));
	}
	let sig = if sig > 0 {
		Some(Signal::try_from(sig as u32)?)
	} else {
		None
	};

	cli!();

	send_signal(pid, sig)?;

	{
		let proc_mutex = Process::current_assert();
		let mut proc = proc_mutex.lock();

		// POSIX requires that at least one pending signal is executed before returning
		if proc.has_signal_pending() {
			// Setting the return value of the system call to `0` after executing a signal
			let mut return_regs = regs.clone();
			return_regs.eax = 0;
			proc.regs = return_regs;

			// Set the process to execute the signal action
			proc.signal_next();
		}
	}

	util::handle_proc_state();

	Ok(0)
}
