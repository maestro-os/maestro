//! This module implements the `setpgid` system call, which allows to set the process group ID of a
//! process.

use crate::errno::Errno;
use crate::errno;
use crate::process::Process;
use crate::process::pid::Pid;
use crate::util::lock::mutex::TMutex;
use crate::util;

/// The implementation of the `getpgid` syscall.
pub fn setpgid(proc: &mut Process, regs: &util::Regs) -> Result<i32, Errno> {
	let pid = regs.ebx as Pid;
	let pgid = regs.ecx as Pid;

	// TODO Check processes SID

	if pid == 0 || pid == proc.get_pid() {
		proc.set_pgid(pgid)?;
	} else {
		let mut mutex = {
			if let Some(proc) = Process::get_by_pid(pid) {
				proc
			} else {
				return Err(errno::ESRCH);
			}
		};
		let mut guard = mutex.lock();
		let proc = guard.get_mut();
		proc.set_pgid(pgid)?;
	}

	Ok(0)
}
