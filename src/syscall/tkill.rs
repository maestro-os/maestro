//! The tkill system call allows to send a signal to a specific thread.

use crate::{
	errno,
	errno::Errno,
	process::{pid::Pid, signal::Signal, Process},
};
use core::ffi::c_int;
use macros::syscall;

#[syscall]
pub fn tkill(tid: Pid, sig: c_int) -> Result<i32, Errno> {
	if sig < 0 {
		return Err(errno!(EINVAL));
	}
	let signal = Signal::try_from(sig as u32)?;

	let proc_mutex = Process::current_assert();
	let mut proc = proc_mutex.lock();

	// Check if the thread to kill is the current
	if proc.tid == tid {
		proc.kill(&signal, false);
	} else {
		// Get the thread
		let thread_mutex = Process::get_by_tid(tid).ok_or(errno!(ESRCH))?;
		let mut thread = thread_mutex.lock();

		// Check permissions
		if !proc.access_profile.can_kill(&thread) {
			return Err(errno!(EPERM));
		}

		thread.kill(&signal, false);
	}

	Ok(0)
}
