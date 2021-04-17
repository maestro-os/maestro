/// This module implements the `setpgid` system call, which allows to set the process group ID of a
/// process.

use crate::process::pid::Pid;
use crate::errno::Errno;
use crate::errno;
use crate::process::Process;
use crate::util;

/// TODO doc
fn handle_setpgid(pid: Pid, pgid: Pid) -> Result<(), Errno> {
	let mut proc = {
		if pid == 0 {
			Process::get_current().unwrap()
		} else {
			if let Some(proc) = Process::get_by_pid(pid) {
				proc
			} else {
				return Err(errno::ESRCH);
			}
		}
	}.lock().get();

	// TODO Check processes SID

	proc.set_pgid(pgid)
}

/// The implementation of the `getpgid` syscall.
pub fn setpgid(regs: &util::Regs) -> u32 {
	let pid = regs.ebx as Pid;
	let pgid = regs.ecx as Pid;

	if let Err(errno) = handle_setpgid(pid, pgid) {
		-errno as _
	} else {
		0
	}
}
