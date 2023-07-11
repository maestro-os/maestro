//! The `getuid32` syscall returns the UID of the process's owner.

use crate::errno::Errno;
use crate::process::Process;
use macros::syscall;

#[syscall]
pub fn getuid32() -> Result<i32, Errno> {
	let proc_mutex = Process::current_assert();
	let proc = proc_mutex.lock();

	Ok(proc.uid as _)
}
