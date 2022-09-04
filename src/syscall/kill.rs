//! This module implements the `kill` system call, which allows to send a signal to a process.

use crate::errno::Errno;
use crate::errno;
use crate::process::Process;
use crate::process::pid::Pid;
use crate::process::regs::Regs;
use crate::process::signal::Signal;
use crate::process::state::State;
use crate::process;

/// Tries to kill the process with PID `pid` with the signal `sig`.
/// If `sig` is None, the function doesn't send a signal, but still checks if there is a process
/// that could be killed.
fn try_kill(pid: Pid, sig: &Option<Signal>) -> Result<(), Errno> {
	let curr_mutex = Process::get_current().unwrap();
	let curr_guard = curr_mutex.lock();
	let curr_proc = curr_guard.get_mut();

	let uid = curr_proc.get_uid();
	let euid = curr_proc.get_euid();

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

	if pid == curr_proc.get_pid() {
		f(curr_proc)?;
	} else {
		let target_mutex = Process::get_by_pid(pid).ok_or_else(|| errno!(ESRCH))?;
		let target_guard = target_mutex.lock();
		let target_proc = target_guard.get_mut();

		f(target_proc)?;
	}

	Ok(())
}

/// Tries to kill a process group.
/// `pid` TODO doc
/// `sig` is the signal to send.
/// If `sig` is None, the function doesn't send a signal, but still checks if there is a process
/// that could be killed.
fn try_kill_group(pid: i32, sig: &Option<Signal>) -> Result<(), Errno> {
	let pgid = match pid {
		0 => {
			let curr_mutex = Process::get_current().unwrap();
			let curr_guard = curr_mutex.lock();
			let curr_proc = curr_guard.get_mut();

			curr_proc.get_pgid()
		},

		i if i < 0 => -pid as Pid,
		_ => pid as Pid,
	};

	// Killing process group
	{
		let mutex = Process::get_by_pid(pgid).ok_or_else(|| errno!(ESRCH))?;
		let guard = mutex.lock();
		let proc = guard.get_mut();

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
/// If `sig` is None, the function doesn't send a signal, but still checks if there is a process
/// that could be killed.
fn send_signal(pid: i32, sig: Option<Signal>) -> Result<(), Errno> {
	if pid > 0 {
		// Kill the process with the given PID
		try_kill(pid as _, &sig)
	} else if pid == 0 {
		// Kill all processes in the current process group
		try_kill_group(0, &sig)
	} else if pid == -1 {
		// Kill all processes for which the current process has the permission
		let scheduler_guard = process::get_scheduler().lock();
		let scheduler = scheduler_guard.get_mut();

		for (pid, _) in scheduler.iter_process() {
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

/// Updates the execution flow of the current process according to its state.
fn handle_state() {
	loop {
		cli!();

		let mutex = Process::get_current().unwrap();
		let guard = mutex.lock();
		let proc = guard.get_mut();

		match proc.get_state() {
			// The process is executing a signal handler. Make the scheduler jump to it
			State::Running => {
				if proc.is_handling_signal() {
					let regs = proc.get_regs().clone();

					drop(guard);
					drop(mutex);

					unsafe {
						regs.switch(true);
					}
				} else {
					return;
				}
			}

			// The process has been stopped. Waiting until wakeup
			State::Stopped => {
				drop(guard);
				drop(mutex);

				crate::wait();
			}

			// The process has been killed. Stopping execution and waiting for the next tick
			State::Zombie => {
				drop(guard);
				drop(mutex);

				crate::enter_loop();
			}

			_ => {}
		}
	}
}

/// The implementation of the `kill` syscall.
pub fn kill(regs: &Regs) -> Result<i32, Errno> {
	let pid = regs.ebx as i32;
	let sig = regs.ecx as i32;

	if sig < 0 {
		return Err(errno!(EINVAL));
	}
	let sig = if sig > 0 {
		Some(Signal::from_id(sig as _)?)
	} else {
		None
	};

	cli!();

	send_signal(pid, sig)?;

	{
		let mutex = Process::get_current().unwrap();
		let guard = mutex.lock();
		let proc = guard.get_mut();

		// POSIX requires that at least one pending signal is executed before returning
		if proc.has_signal_pending() {
			// Setting the return value of the system call to `0` after executing a signal
			let mut return_regs = regs.clone();
			return_regs.eax = 0;
			proc.set_regs(return_regs);

			// Set the process to execute the signal action
			proc.signal_next();
		}
	}

	handle_state();

	Ok(0)
}
