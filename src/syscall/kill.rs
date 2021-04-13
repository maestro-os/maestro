/// This module implements the `kill` system call, which allows to send a signal to a process.

use crate::errno;
use crate::process::State;
use crate::process::pid::Pid;
use crate::process::Process;
use crate::util;

/// The implementation of the `kill` syscall.
pub fn kill(regs: &util::Regs) -> u32 {
	let pid = regs.ebx as i32;
	let sig = regs.ecx as u8;
	// TODO Handle sig == 0

	if pid > 0 {
		if let Some(mut proc) = Process::get_by_pid(pid as Pid) {
			if proc.get_state() != State::Zombie {
				if let Err(errno) = proc.kill(sig) {
					-errno as _
				} else {
					0
				}
			} else {
				-errno::ESRCH as _
			}
		} else {
			-errno::ESRCH as _
		}
	} else if pid == 0 {
		// TODO Send to every processes in the process group
		-errno::ESRCH as _
	} else if pid == -1 {
		// TODO Send to every processes that the process has permission to send a signal to
		-errno::ESRCH as _
	} else {
		// TODO Send to process group id `-pid`
		-errno::ESRCH as _
	}
}
