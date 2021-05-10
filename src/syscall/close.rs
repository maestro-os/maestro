//! TODO doc

use crate::errno::Errno;
use crate::errno;
use crate::process::Process;
use crate::util;

/// The implementation of the `close` syscall.
pub fn close(proc: &mut Process, regs: &util::Regs) -> Result<i32, Errno> {
	let fd = regs.ebx;

	if proc.close_fd(fd).is_ok() {
		Ok(0)
	} else {
		Err(errno::EBADF)
	}
}
