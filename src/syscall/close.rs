//! The `close` system call closes the given file descriptor.

use crate::errno;
use crate::errno::Errno;
use crate::process::regs::Regs;
use crate::process::Process;

/// The implementation of the `close` syscall.
pub fn close(regs: &Regs) -> Result<i32, Errno> {
	let fd = regs.ebx;

	let mutex = Process::get_current().unwrap();
	let guard = mutex.lock();
	let proc = guard.get_mut();

	if proc.close_fd(fd).is_ok() {
		Ok(0)
	} else {
		Err(errno!(EBADF))
	}
}
