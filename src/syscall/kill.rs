/// This module implements the `kill` system call, which allows to send a signal to a process.

use crate::errno::Errno;
use crate::errno;
use crate::process::Process;
use crate::process::State;
use crate::process::pid::Pid;
use crate::util;

/// TODO doc
fn handle_kill(pid: i32, sig: u8) -> Result<(), Errno> {
	// TODO Handle sig == 0
	// TODO Handle when killing current process

	if pid > 0 {
		if let Some(mut proc) = Process::get_by_pid(pid as Pid) {
			if proc.get_state() != State::Zombie {
				proc.kill(sig)
			} else {
				Err(errno::ESRCH)
			}
		} else {
			Err(errno::ESRCH)
		}
	} else if pid == 0 {
		// TODO Send to every processes in the process group
		Err(errno::ESRCH)
	} else if pid == -1 {
		// TODO Send to every processes that the process has permission to send a signal to
		Err(errno::ESRCH)
	} else {
		// TODO Send to process group id `-pid`
		Err(errno::ESRCH)
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
