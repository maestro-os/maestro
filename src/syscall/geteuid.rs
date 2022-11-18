//! The `geteuid` syscall returns the effective UID of the process's owner.

use crate::errno::Errno;
use crate::process::Process;
use macros::syscall;

/// The implementation of the `geteuid` syscall.
#[syscall]
pub fn geteuid() -> Result<i32, Errno> {
	let mutex = Process::get_current().unwrap();
	let guard = mutex.lock();
	let proc = guard.get_mut();

	Ok(proc.get_euid() as _)
}
