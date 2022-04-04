//! The `getegid` syscall returns the effective GID of the process's owner.

use crate::errno::Errno;
use crate::process::Process;
use crate::process::regs::Regs;

/// The implementation of the `getegid` syscall.
pub fn getegid(_: &Regs) -> Result<i32, Errno> {
	let mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock();
	let proc = guard.get_mut();

	Ok(proc.get_egid() as _)
}
