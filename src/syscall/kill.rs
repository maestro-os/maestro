//! This module implements the `kill` system call, which allows to send a signal to a process.

use crate::errno::Errno;
use crate::errno;
use crate::process::Process;
use crate::process::State;
use crate::process::pid::Pid;
use crate::process::signal::Signal;
use crate::process;
use crate::util;

/// Tries to kill the process with PID `pid` with the signal `sig`.
/// If `sig` is None, the function doesn't send a signal, but still checks if there is a process
/// that could be killed.
fn try_kill(pid: i32, sig: Option<Signal>) -> Result<i32, Errno> {
	if let Some(mut proc) = Process::get_by_pid(pid as Pid) {
		let mut guard = proc.lock(false);
		let proc = guard.get_mut();

		if proc.get_state() != State::Zombie {
			if let Some(sig) = sig {
				proc.kill(sig);
			}
			Ok(0)
		} else {
			Err(errno::ESRCH)
		}
	} else {
		Err(errno::ESRCH)
	}
}

/// Sends the signal `sig` to the processes according to the given value `pid`.
/// `proc` is the current process.
/// If `sig` is None, the function doesn't send a signal, but still checks if there is a process
/// that could be killed.
fn send_signal(pid: i32, sig: Option<Signal>, proc: &mut Process) -> Result<i32, Errno> {
	if pid == proc.get_pid() as _ {
		if let Some(sig) = sig {
			proc.kill(sig);
		}
		Ok(0)
	} else if pid > 0 {
		try_kill(pid, sig)
	} else if pid == 0 || -pid as Pid == proc.get_pid() {
		let group = proc.get_group_processes();

		if let Some(sig) = sig {
			for p in group {
				try_kill(*p as _, Some(sig.clone())).unwrap();
			}
		}

		if !group.is_empty() {
			Ok(0)
		} else {
			Err(errno::ESRCH)
		}
	} else if pid == -1 {
		// TODO Send to every processes that the process has permission to send a signal to
		todo!();
	} else {
		if let Some(mut proc) = Process::get_by_pid(-pid as _) {
			let mut guard = proc.lock(false);
			let proc = guard.get_mut();
			let group = proc.get_group_processes();

			if let Some(sig) = sig {
				for p in group {
					try_kill(*p as _, Some(sig.clone())).unwrap();
				}
			}

			if !group.is_empty() {
				Ok(0)
			} else {
				Err(errno::ESRCH)
			}
		} else {
			Err(errno::ESRCH)
		}
	}
}

/// The implementation of the `kill` syscall.
pub fn kill(regs: &util::Regs) -> Result<i32, Errno> {
	let pid = regs.ebx as i32;
	let sig = regs.ecx as i32;

	// TODO Check permission (with real or effective UID)

	cli!();

	let sig = {
		if sig > 0 {
			Some(Signal::new(sig)?)
		} else {
			None
		}
	};

	let mut mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock(false);
	let proc = guard.get_mut();

	send_signal(pid, sig, proc)?;

	// POSIX requires that at least one pending signal is executed before returning
	if proc.has_signal_pending() {
		// Set the process to execute the signal action
		proc.signal_next();
	}

	// Getting process's information and dropping the guard to avoid deadlocks
	let state = proc.get_state();
	drop(guard);

	match state {
		// The process is executing a signal handler. Make the scheduler jump to it
		process::State::Running => crate::wait(),

		// The process has been stopped. Waiting until wakeup
		process::State::Stopped => crate::wait(),

		// The process has been killed. Stopping execution and waiting for the next tick
		process::State::Zombie => crate::enter_loop(),

		_ => {},
	}

	Ok(0)
}
