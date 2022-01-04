//! The `gettid` system call returns the thread ID of the current process.

use crate::errno::Errno;
use crate::process::Process;
use crate::process::Regs;

/// The implementation of the `gettid` syscall.
pub fn gettid(_regs: &Regs) -> Result<i32, Errno> {
	let mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock();
	let proc = guard.get_mut();

	Ok(proc.get_tid() as _)
}
