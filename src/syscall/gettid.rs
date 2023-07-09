//! The `gettid` system call returns the thread ID of the current process.

use crate::errno::Errno;
use crate::process::Process;
use macros::syscall;

#[syscall]
pub fn gettid() -> Result<i32, Errno> {
	let proc_mutex = Process::current_assert();
	let proc = proc_mutex.lock();

	Ok(proc.tid as _)
}
