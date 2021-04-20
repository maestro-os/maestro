/// TODO doc

use crate::errno;
use crate::process::Process;
use crate::util;

/// The implementation of the `close` syscall.
pub fn close(proc: &mut Process, regs: &util::Regs) -> u32 {
	let fd = regs.ebx;

	if proc.close_fd(fd).is_ok() {
		0
	} else {
		-errno::EBADF as _
	}
}
