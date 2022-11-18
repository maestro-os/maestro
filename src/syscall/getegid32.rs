//! The `getegid32` syscall returns the effective GID of the process's owner.

use crate::errno::Errno;
use crate::process::Process;
use macros::syscall;

/// The implementation of the `getegid32` syscall.
#[syscall]
pub fn getegid32() -> Result<i32, Errno> {
	let mutex = Process::get_current().unwrap();
	let guard = mutex.lock();
	let proc = guard.get_mut();

	Ok(proc.get_egid() as _)
}
