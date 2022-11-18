//! The `getpid` system call returns the PID of the current process.

use crate::errno::Errno;
use crate::process::Process;
use macros::syscall;

/// The implementation of the `getpid` syscall.
#[syscall]
pub fn getpid() -> Result<i32, Errno> {
	let mutex = Process::get_current().unwrap();
	let guard = mutex.lock();
	let proc = guard.get_mut();

	Ok(proc.get_pid() as _)
}
