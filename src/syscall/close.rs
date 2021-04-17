/// TODO doc

use crate::errno;
use crate::process::Process;
use crate::util::lock::mutex::MutMutexGuard;
use crate::util;

/// The implementation of the `close` syscall.
pub fn close(regs: &util::Regs) -> u32 {
	let fd = regs.ebx;

	let mut mutex = Process::get_current().unwrap();
	let mut guard = MutMutexGuard::new(&mut mutex);
	let curr_proc = guard.get_mut();
	if curr_proc.close_fd(fd).is_ok() {
		0
	} else {
		-errno::EBADF as _
	}
}
