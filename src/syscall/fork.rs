//! The `fork` system call duplicates the whole current process into a new child
//! process. Execution resumes at the same location for both processes but the
//! return value is different to allow differentiation.

use crate::errno::Errno;
use crate::process::ForkOptions;
use crate::process::Process;
use macros::syscall;

#[syscall]
pub fn fork() -> Result<i32, Errno> {
	// The current process
	let curr_mutex = Process::get_current().unwrap();
	// A weak pointer to the new process's parent
	let parent = curr_mutex.new_weak();

	let curr_proc = curr_mutex.lock();

	let new_mutex = curr_proc.fork(parent, ForkOptions::default())?;
	let new_proc = new_mutex.lock();

	// Setting registers
	let mut regs = regs.clone();
	// Setting return value to `0`
	regs.eax = 0;
	new_proc.set_regs(regs);

	Ok(new_proc.get_pid() as _)
}
