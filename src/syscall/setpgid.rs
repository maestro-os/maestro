//! This module implements the `setpgid` system call, which allows to set the
//! process group ID of a process.

use crate::errno;
use crate::errno::Errno;
use crate::process::pid::Pid;
use crate::process::Process;

/// The implementation of the `setpgid` syscall.
pub fn setpgid(pid: Pid, pgid: Pid) -> Result<i32, Errno> {
	// TODO Check processes SID

	let mutex = Process::get_current().unwrap();
	let guard = mutex.lock();
	let proc = guard.get_mut();

	if pid == 0 {
		pid = proc.get_pid();
	}
	if pgid == 0 {
		pgid = pid;
	}

	if pid == proc.get_pid() {
		proc.set_pgid(pgid)?;
	} else {
		drop(guard);

		let mutex = Process::get_by_pid(pid).ok_or_else(|| errno!(ESRCH))?;
		let guard = mutex.lock();
		let proc = guard.get_mut();

		proc.set_pgid(pgid)?;
	}

	Ok(0)
}
