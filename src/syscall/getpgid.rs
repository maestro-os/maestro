//! This module implements the `getpgid` system call, which allows to get the
//! process group ID of a process.

use crate::errno;
use crate::errno::Errno;
use crate::process::pid::Pid;
use crate::process::Process;
use macros::syscall;

#[syscall]
pub fn getpgid(pid: Pid) -> Result<i32, Errno> {
	let proc_mutex = Process::current_assert();
	let proc = proc_mutex.lock();

	if pid == 0 {
		Ok(proc.pgid as _)
	} else {
		let proc_mutex = {
			if let Some(proc) = Process::get_by_pid(pid) {
				proc
			} else {
				return Err(errno!(ESRCH));
			}
		};
		let proc = proc_mutex.lock();

		Ok(proc.pgid as _)
	}
}
