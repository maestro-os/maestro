//! The `vfork` system call works the same as the `fork` system call, except the
//! parnet process is blocked until the child process exits or executes a
//! program. During that time, the child process also shares the same memory
//! space as the parent.

use crate::errno::Errno;
use crate::process::scheduler;
use crate::process::ForkOptions;
use crate::process::Process;
use crate::util::ptr::arc::Arc;
use macros::syscall;

#[syscall]
pub fn vfork() -> Result<i32, Errno> {
	let new_pid = {
		// The current process
		let curr_mutex = Process::current_assert();
		// A weak pointer to the new process's parent
		let parent = Arc::downgrade(&curr_mutex);

		let mut curr_proc = curr_mutex.lock();

		let fork_options = ForkOptions {
			vfork: true,
			..ForkOptions::default()
		};
		let new_mutex = curr_proc.fork(parent, fork_options)?;
		let mut new_proc = new_mutex.lock();

		// Setting registers
		let mut regs = regs.clone();
		// Setting return value to `0`
		regs.eax = 0;
		new_proc.regs = regs;

		new_proc.pid
	};

	// Letting another process run instead of the current. Because the current
	// process must now wait for the child process to terminate or execute a program
	scheduler::end_tick();

	Ok(new_pid as _)
}
