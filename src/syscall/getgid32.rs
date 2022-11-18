//! The `getgid32` syscall returns the GID of the process's owner.

use crate::errno::Errno;
use crate::process::Process;
use macros::syscall;

/// The implementation of the `getgid32` syscall.
#[syscall]
pub fn getgid32() -> Result<i32, Errno> {
	let mutex = Process::get_current().unwrap();
	let guard = mutex.lock();
	let proc = guard.get_mut();

	Ok(proc.get_gid() as _)
}
