//! This module implements the `kill` system call, which allows to send a signal to a process.

use crate::errno::Errno;
use crate::errno;
use crate::file::Uid;
use crate::process::Process;
use crate::process::Regs;
use crate::process::State;
use crate::process::pid::Pid;
use crate::process::signal::Signal;
use crate::process;

/// Tries to kill the process with PID `pid` with the signal `sig`.
/// `euid` is the effective user ID of the sender process.
/// If `sig` is None, the function doesn't send a signal, but still checks if there is a process
/// that could be killed.
fn try_kill(pid: i32, sig: Option<Signal>, euid: Uid) -> Result<i32, Errno> {
	if let Some(proc) = Process::get_by_pid(pid as Pid) {
		let mut guard = proc.lock();
		let proc = guard.get_mut();

		if proc.get_state() != State::Zombie {
			if euid == proc.get_uid() || euid == proc.get_euid() {
				if let Some(sig) = sig {
					proc.kill(sig, false);
				}

				Ok(0)
			} else {
				Err(errno::EPERM)
			}
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
			proc.kill(sig, false);
		}
		Ok(0)
	} else if pid > 0 {
		try_kill(pid, sig, proc.get_euid())
	} else if pid == 0 || -pid as Pid == proc.get_pid() {
		let group = proc.get_group_processes();

		if let Some(sig) = sig {
			for p in group {
				try_kill(*p as _, Some(sig.clone()), proc.get_euid()).unwrap();
			}
		}

		if !group.is_empty() {
			Ok(0)
		} else {
			Err(errno::ESRCH)
		}
	} else if pid == -1 {
		let mut scheduler_guard = process::get_scheduler().lock();
		let scheduler = scheduler_guard.get_mut();

		// Variable telling whether at least one process is killed
		let mut found = false;

		scheduler.foreach_process(| pid, p | {
			if *pid != process::pid::INIT_PID && *pid != proc.get_pid() {
				if let Some(sig) = &sig {
					let mut proc_guard = p.lock();
					let p = proc_guard.get_mut();

					if proc.get_euid() == p.get_uid() || proc.get_euid() == p.get_euid() {
						p.kill(sig.clone(), false);
						found = true;
					}
				}
			}
		});

		if found {
			Ok(0)
		} else {
			Err(errno::ESRCH)
		}
	} else {
		if let Some(proc) = Process::get_by_pid(-pid as _) {
			let mut guard = proc.lock();
			let proc = guard.get_mut();
			let group = proc.get_group_processes();

			if let Some(sig) = sig {
				for p in group {
					try_kill(*p as _, Some(sig.clone()), proc.get_euid()).unwrap();
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

/// Updates the execution flow of the current process according to its state.
fn handle_state() {
	loop {
		cli!();

		let mutex = Process::get_current().unwrap();
		let mut guard = mutex.lock();
		let proc = guard.get_mut();

		match proc.get_state() {
			// The process is executing a signal handler. Make the scheduler jump to it
			process::State::Running => {
				if proc.is_handling_signal() {
					let regs = proc.get_regs().clone();
					drop(guard);

					unsafe {
						regs.switch(true);
					}
				} else {
					return;
				}
			},

			// The process has been stopped. Waiting until wakeup
			process::State::Stopped => {
				drop(guard);
				crate::wait();
			},

			// The process has been killed. Stopping execution and waiting for the next tick
			process::State::Zombie => {
				drop(guard);
				crate::enter_loop();
			},

			_ => {},
		}
	}
}

/// The implementation of the `kill` syscall.
pub fn kill(regs: &Regs) -> Result<i32, Errno> {
	let pid = regs.ebx as i32;
	let sig = regs.ecx as i32;

	cli!();

	let sig = {
		if sig > 0 {
			Some(Signal::new(sig)?)
		} else {
			None
		}
	};

	{
		let mutex = Process::get_current().unwrap();
		let mut guard = mutex.lock();
		let proc = guard.get_mut();

		send_signal(pid, sig, proc)?;

		// POSIX requires that at least one pending signal is executed before returning
		if proc.has_signal_pending() {
			// Setting the return value of the system call to `0` after executing a signal
			let mut return_regs = regs.clone();
			return_regs.eax = 0;
			proc.set_regs(&return_regs);

			// Set the process to execute the signal action
			proc.signal_next();
		}
	}

	handle_state();

	Ok(0)
}
