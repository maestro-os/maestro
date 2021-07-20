//! The `dup2` syscall allows to duplicate a file descriptor, specifying the id of the newly
//! created file descriptor.

use crate::errno::Errno;
use crate::process::Process;
use crate::util;

/// The implementation of the `dup2` syscall.
pub fn dup2(regs: &util::Regs) -> Result<i32, Errno> {
	let oldfd = regs.ebx;
	let newfd = regs.ecx;

	let mut mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock(false);
	let proc = guard.get_mut();

	let newfd = proc.duplicate_fd(oldfd, Some(newfd))?;
	Ok(newfd.get_id() as _)
}
