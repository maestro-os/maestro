//! The `gettid` system call returns the thread ID of the current process.

use crate::errno::Errno;
use crate::process::Process;
use macros::syscall;

#[syscall]
pub fn gettid() -> Result<i32, Errno> {
	let mutex = Process::get_current().unwrap();
	let guard = mutex.lock();
	let proc = guard.get_mut();

	Ok(proc.get_tid() as _)
}
