//! The `getgid` syscall returns the GID of the process's owner.

use crate::errno::Errno;
use crate::process::Process;
use crate::process::Regs;

/// The implementation of the `getgid` syscall.
pub fn getgid(_: &Regs) -> Result<i32, Errno> {
	let mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock();
	let proc = guard.get_mut();

	Ok(proc.get_gid() as _)
}
