//! The `vfork` system call works the same as the `fork` system call, except the parnet process is
//! blocked until the child process exits or executes a program. During that time, the child
//! process also shares the same memory space as the parent.

use crate::errno::Errno;
use crate::process::ForkOptions;
use crate::process::Process;
use crate::process::regs::Regs;

/// The implementation of the `vfork` syscall.
pub fn vfork(_regs: &Regs) -> Result<i32, Errno> {
	let new_pid = {
		// The current process
		let curr_mutex = Process::get_current().unwrap();
		// A weak pointer to the new process's parent
		let parent = curr_mutex.new_weak();

		let mut curr_guard = curr_mutex.lock();
		let curr_proc = curr_guard.get_mut();

		let fork_options = ForkOptions {
			vfork: true,
			..ForkOptions::default()
		};
		let new_mutex = curr_proc.fork(parent, fork_options)?;
		let mut new_guard = new_mutex.lock();
		let new_proc = new_guard.get_mut();

		// Setting registers
		let mut regs = curr_proc.get_regs().clone();
		// Setting return value to `0`
		regs.eax = 0;
		new_proc.set_regs(regs);

		new_proc.get_pid()
	};

	// Letting another process run instead of the current. Because the current process must now
	// wait for the child process to terminate or execute a program
	crate::wait();

	Ok(new_pid as _)
}
