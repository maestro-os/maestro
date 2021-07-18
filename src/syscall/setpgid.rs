//! This module implements the `setpgid` system call, which allows to set the process group ID of a
//! process.

use crate::errno::Errno;
use crate::errno;
use crate::process::Process;
use crate::process::pid::Pid;
use crate::util;

/// The implementation of the `setpgid` syscall.
pub fn setpgid(proc: &mut Process, regs: &util::Regs) -> Result<i32, Errno> {
	let mut pid = regs.ebx as Pid;
	let mut pgid = regs.ecx as Pid;

	// TODO Check processes SID

	if pid == 0 {
		pid = proc.get_pid();
	}
	if pgid == 0 {
		pgid = pid;
	}

	if pid == proc.get_pid() {
		proc.set_pgid(pgid)?;
	} else {
		let mut mutex = {
			if let Some(proc) = Process::get_by_pid(pid) {
				proc
			} else {
				return Err(errno::ESRCH);
			}
		};
		let mut guard = mutex.lock(false);
		let proc = guard.get_mut();
		proc.set_pgid(pgid)?;
	}

	Ok(0)
}
