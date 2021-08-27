//! The `fork` system call duplicates the whole current process into a new child process. Execution
//! resumes at the same location for both processes but the return value is different to allow
//! differentiation.

use crate::errno::Errno;
use crate::process::Process;
use crate::util;

/// The implementation of the `fork` syscall.
pub fn fork(regs: &util::Regs) -> Result<i32, Errno> {
	let mut new_mutex = {
		// The current process
		let mut curr_mutex = Process::get_current().unwrap();
		// A weak pointer to the new process's parent
		let parent = curr_mutex.new_weak();

		let mut curr_guard = curr_mutex.lock(false);
		let curr_proc = curr_guard.get_mut();

		curr_proc.set_regs(regs);
		curr_proc.fork(parent)?
	};
	let mut new_guard = new_mutex.lock(false);
	let new_proc = new_guard.get_mut();

	Ok(new_proc.get_pid() as _)
}
