//! This module implements the `setpgid` system call, which allows to set the
//! process group ID of a process.

use crate::errno;
use crate::errno::Errno;
use crate::process::pid::Pid;
use crate::process::Process;
use macros::syscall;

#[syscall]
pub fn setpgid(pid: Pid, pgid: Pid) -> Result<i32, Errno> {
	let mut pid = pid;
	let mut pgid = pgid;

	// TODO Check processes SID

	let proc_mutex = Process::get_current().unwrap();
	let proc = proc_mutex.lock();

	if pid == 0 {
		pid = proc.get_pid();
	}
	if pgid == 0 {
		pgid = pid;
	}

	if pid == proc.get_pid() {
		proc.set_pgid(pgid)?;
	} else {
		drop(proc);

		let proc_mutex = Process::get_by_pid(pid).ok_or_else(|| errno!(ESRCH))?;
		let proc = proc_mutex.lock();

		proc.set_pgid(pgid)?;
	}

	Ok(0)
}
