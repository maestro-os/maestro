//! This module implements the `kill` system call, which allows to send a signal to a process.

use crate::errno::Errno;
use crate::errno;
use crate::process::Process;
use crate::process::State;
use crate::process::pid::Pid;
use crate::process::signal::Signal;
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

/// The implementation of the `kill` syscall.
pub fn kill(regs: &util::Regs) -> Result<i32, Errno> {
	let pid = regs.ebx as i32;
	let sig = regs.ecx as u8;

	// TODO Handle sig == 0
	// TODO Check permission (with real or effective UID)
	// TODO Handle when killing current process (execute before returning)

	let sig = Signal::new(sig)?;

	let mut mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock(false);
	let proc = guard.get_mut();

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
