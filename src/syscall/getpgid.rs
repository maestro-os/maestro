/// This module implements the `getpgid` system call, which allows to get the process group ID of a
/// process.

use crate::process::pid::Pid;
use crate::errno::Errno;
use crate::errno;
use crate::process::Process;
use crate::util;

/// TODO doc
fn handle_getpgid(pid: Pid) -> Result<Pid, Errno> {
	let proc = {
		if pid == 0 {
			Process::get_current().unwrap()
		} else {
			if let Some(proc) = Process::get_by_pid(pid) {
				proc
			} else {
				return Err(errno::ESRCH);
			}
		}
	};

	Ok(proc.get_pgid())
}

/// The implementation of the `getpgid` syscall.
pub fn getpgid(regs: &util::Regs) -> u32 {
	let pid = regs.ebx as Pid;

	let r = handle_getpgid(pid);
	if let Ok(pgid) = r {
		pgid as _
	} else {
		-r.unwrap_err() as _
	}
}
