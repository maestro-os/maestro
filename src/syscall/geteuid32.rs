//! The `geteuid32` syscall returns the effective UID of the process's owner.

use crate::errno::Errno;
use crate::process::regs::Regs;
use crate::process::Process;

/// The implementation of the `geteuid32` syscall.
pub fn geteuid32(_: &Regs) -> Result<i32, Errno> {
	let mutex = Process::get_current().unwrap();
	let guard = mutex.lock();
	let proc = guard.get_mut();

	Ok(proc.get_euid() as _)
}
