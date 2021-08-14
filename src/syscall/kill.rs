//! This module implements the `kill` system call, which allows to send a signal to a process.

use crate::errno::Errno;
use crate::errno;
use crate::gdt;
use crate::process::Process;
use crate::process::State;
use crate::process::pid::Pid;
use crate::process::scheduler;
use crate::process::signal::Signal;
use crate::process;
use crate::util;

/// Tries to kill the process with PID `pid` with the signal `sig`.
fn try_kill(pid: i32, sig: Signal) -> Result<i32, Errno> {
	if let Some(mut proc) = Process::get_by_pid(pid as Pid) {
		let mut guard = proc.lock(false);
		let proc = guard.get_mut();

		if proc.get_state() != State::Zombie {
			proc.kill(sig);
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
fn send_signal(pid: i32, sig: Signal, proc: &mut Process) -> Result<i32, Errno> {
	if pid == proc.get_pid() as _ {
		proc.kill(sig);
		Ok(0)
	} else if pid > 0 {
		try_kill(pid, sig)
	} else if pid == 0 {
		for p in proc.get_group_processes() {
			try_kill(*p as _, sig.clone()).unwrap();
		}

		proc.kill(sig);
		Ok(0)
	} else if pid == -1 {
		// TODO Send to every processes that the process has permission to send a signal to
		todo!();
	} else {
		if -pid == proc.get_pid() as _ {
			for p in proc.get_group_processes() {
				try_kill(*p as _, sig.clone()).unwrap();
			}

			proc.kill(sig);
			return Ok(0);
		} else if let Some(mut proc) = Process::get_by_pid(-pid as _) {
			let mut guard = proc.lock(false);
			let proc = guard.get_mut();
			for p in proc.get_group_processes() {
				try_kill(*p as _, sig.clone()).unwrap();
			}

			proc.kill(sig);
			return Ok(0);
		}

		Err(errno::ESRCH)
	}
}

/// The implementation of the `kill` syscall.
pub fn kill(regs: &util::Regs) -> Result<i32, Errno> {
	let pid = regs.ebx as i32;
	let sig = regs.ecx as i32;

	// TODO Handle sig == 0
	// TODO Check permission (with real or effective UID)
	// TODO Handle when killing current process (execute before returning)

	cli!();

	let sig = Signal::new(sig)?;

	let mut mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock(false);
	let proc = guard.get_mut();

	send_signal(pid, sig, proc)?;

	// POSIX requires that at least one pending signal is executed before returning
	if proc.has_signal_pending() {
		// Set the process's registers to make it resume execution after the current syscall
		let mut regs = regs.clone();
		regs.eax = 0;
		proc.set_regs(&regs);
		// Set the process to execute the signal action
		proc.signal_next();

		// Getting process's information and dropping the guard avoid deadlocks
		let regs = proc.get_regs().clone();
		let state = proc.get_state();
		drop(guard);

		match state {
			// The process is executing a signal handler. Jump directly to it
			process::State::Running => {
				unsafe {
					scheduler::context_switch(&regs,
						(gdt::USER_DATA_OFFSET | 3) as _,
						(gdt::USER_CODE_OFFSET | 3) as _);
				}
			},

			// The process has been stopped. Waiting until wakeup
			process::State::Stopped => {
				loop {
					crate::wait();

					let mut mutex = Process::get_current().unwrap();
					let guard = mutex.lock(false);
					let proc = guard.get();
					if proc.get_state() != process::State::Stopped {
						break;
					}
				}
			},

			// The process has been killed. Stopping execution and waiting for the next tick
			process::State::Zombie => crate::enter_loop(),

			_ => {},
		}
	}

	Ok(0)
}
