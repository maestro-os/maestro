/// TODO doc

use crate::errno;
use crate::process::Process;
use crate::util;

/// The implementation of the `close` syscall.
pub fn close(regs: &util::Regs) -> u32 {
	let fd = regs.ebx;

	let mut curr_proc = Process::get_current().unwrap().lock().get();
	if curr_proc.close_fd(fd).is_ok() {
		0
	} else {
		-errno::EBADF as _
	}
}
