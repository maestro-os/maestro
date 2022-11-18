//! This module implements the `getpgid` system call, which allows to get the
//! process group ID of a process.

use crate::errno;
use crate::errno::Errno;
use crate::process::pid::Pid;
use crate::process::Process;
use macros::syscall;

/// The implementation of the `getpgid` syscall.
#[syscall]
pub fn getpgid(pid: Pid) -> Result<i32, Errno> {
	let mutex = Process::get_current().unwrap();
	let guard = mutex.lock();
	let proc = guard.get_mut();

	if pid == 0 {
		Ok(proc.get_pgid() as _)
	} else {
		let mutex = {
			if let Some(proc) = Process::get_by_pid(pid) {
				proc
			} else {
				return Err(errno!(ESRCH));
			}
		};
		let guard = mutex.lock();
		let proc = guard.get_mut();

		Ok(proc.get_pgid() as _)
	}
}
