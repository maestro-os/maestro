//! TODO doc

use crate::errno::Errno;
use crate::process::Process;
use crate::util;

/// The implementation of the `fork` syscall.
pub fn fork(regs: &util::Regs) -> Result<i32, Errno> {
	let mut new_mutex = {
		let mut curr_mutex = Process::get_current().unwrap();
		let mut curr_guard = curr_mutex.lock(false);
		let curr_proc = curr_guard.get_mut();

		curr_proc.set_regs(regs);
		curr_proc.fork()?
	};
	let mut new_guard = new_mutex.lock(false);
	let new_proc = new_guard.get_mut();

	Ok(new_proc.get_pid() as _)
}
