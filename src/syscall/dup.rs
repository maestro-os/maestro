//! The `dup` syscall allows to duplicate a file descriptor.

use crate::errno::Errno;
use crate::file::fd::NewFDConstraint;
use crate::process::Process;
use crate::process::regs::Regs;

/// The implementation of the `dup` syscall.
pub fn dup(regs: &Regs) -> Result<i32, Errno> {
	let oldfd = regs.ebx;

	let mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock();
	let proc = guard.get_mut();

	let (newfd_id, _) = proc.duplicate_fd(oldfd, NewFDConstraint::None, false)?;
	Ok(newfd_id as _)
}
