/// This module implements the `kill` system call, which allows to send a signal to a process.

use crate::errno::Errno;
use crate::errno;
use crate::process::Process;
use crate::process::State;
use crate::process::pid::Pid;
use crate::util::lock::mutex::MutMutexGuard;
use crate::util;

/// Tries to kill the process with PID `pid` with the signal `sig`.
fn try_kill(pid: i32, sig: u8) -> Result<(), Errno> {
	if let Some(mut proc) = Process::get_by_pid(pid as Pid) {
		let mut guard = MutMutexGuard::new(&mut proc);
		let proc = guard.get_mut();
		if proc.get_state() != State::Zombie {
			proc.kill(sig)
		} else {
			Err(errno::ESRCH)
		}
	} else {
		Err(errno::ESRCH)
	}
}

/// TODO doc
fn handle_kill(pid: i32, sig: u8) -> Result<(), Errno> {
	// TODO Handle sig == 0
	// TODO Handle when killing current process (execute before returning)

	if pid > 0 {
		try_kill(pid, sig)
	} else if pid == 0 {
		let mut mutex = Process::get_current().unwrap();
		let mut guard = MutMutexGuard::new(&mut mutex);
		let curr_proc = guard.get_mut();
		for p in curr_proc.get_group_processes() {
			try_kill(*p as _, sig).unwrap();
		}
		curr_proc.kill(sig)
	} else if pid == -1 {
		// TODO Send to every processes that the process has permission to send a signal to
		Err(errno::ESRCH)
	} else {
		if let Some(mut proc) = Process::get_by_pid(-pid as _) {
			let mut guard = MutMutexGuard::new(&mut proc);
			let proc = guard.get_mut();
			for p in proc.get_group_processes() {
				try_kill(*p as _, sig).unwrap();
			}
			proc.kill(sig)
		} else {
			Err(errno::ESRCH)
		}
	}
}

/// The implementation of the `kill` syscall.
pub fn kill(regs: &util::Regs) -> u32 {
	let pid = regs.ebx as i32;
	let sig = regs.ecx as u8;

	if let Err(errno) = handle_kill(pid, sig) {
		-errno as _
	} else {
		0
	}
}
