//! The `getppid` system call returns the PID of the process's parent.

use crate::errno::Errno;
use crate::process::Process;
use crate::process::Regs;

/// The implementation of the `getppid` syscall.
pub fn getppid(_regs: &Regs) -> Result<i32, Errno> {
	let mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock();
	let proc = guard.get_mut();

	Ok(proc.get_parent_pid() as _)
}
