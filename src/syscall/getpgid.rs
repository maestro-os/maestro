//! This module implements the `getpgid` system call, which allows to get the process group ID of a
//! process.

use crate::errno::Errno;
use crate::errno;
use crate::process::Process;
use crate::process::pid::Pid;
use crate::util::lock::mutex::TMutex;
use crate::util;

/// TODO doc
fn handle_getpgid(pid: Pid, proc: &mut Process) -> Result<i32, Errno> {
	if pid == 0 {
		Ok(proc.get_pid() as _)
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
		Ok(proc.get_pgid() as _)
	}
}

/// The implementation of the `getpgid` syscall.
pub fn getpgid(proc: &mut Process, regs: &util::Regs) -> Result<i32, Errno> {
	let pid = regs.ebx as Pid;

	handle_getpgid(pid, proc)
}
