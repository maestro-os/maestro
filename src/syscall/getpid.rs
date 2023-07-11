//! The `getpid` system call returns the PID of the current process.

use crate::errno::Errno;
use crate::process::Process;
use macros::syscall;

#[syscall]
pub fn getpid() -> Result<i32, Errno> {
	let proc_mutex = Process::current_assert();
	let proc = proc_mutex.lock();

	Ok(proc.pid as _)
}
