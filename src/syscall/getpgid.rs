/// This module implements the `getpgid` system call, which allows to get the process group ID of a
/// process.

use crate::errno::Errno;
use crate::errno;
use crate::process::Process;
use crate::process::pid::Pid;
use crate::util::lock::mutex::MutexGuard;
use crate::util;

/// TODO doc
fn handle_getpgid(pid: Pid, proc: &mut Process) -> Result<Pid, Errno> {
	if pid == 0 {
		Ok(proc.get_pid())
	} else {
		let mut mutex = {
			if let Some(proc) = Process::get_by_pid(pid) {
				proc
			} else {
				return Err(errno::ESRCH);
			}
		};
		let mut guard = MutexGuard::new(&mut mutex);
		let proc = guard.get_mut();
		Ok(proc.get_pgid())
	}
}

/// The implementation of the `getpgid` syscall.
pub fn getpgid(proc: &mut Process, regs: &util::Regs) -> u32 {
	let pid = regs.ebx as Pid;

	let r = handle_getpgid(pid, proc);
	if let Ok(pgid) = r {
		pgid as _
	} else {
		-r.unwrap_err() as _
	}
}
