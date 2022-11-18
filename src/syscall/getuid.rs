//! The `getuid` syscall returns the UID of the process's owner.

use crate::errno::Errno;
use crate::process::Process;
use macros::syscall;

/// The implementation of the `getuid` syscall.
#[syscall]
pub fn getuid() -> Result<i32, Errno> {
	let mutex = Process::get_current().unwrap();
	let guard = mutex.lock();
	let proc = guard.get_mut();

	Ok(proc.get_uid() as _)
}
