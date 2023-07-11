//! The tkill system call allows to send a signal to a specific thread.

use crate::errno;
use crate::errno::Errno;
use crate::process::pid::Pid;
use crate::process::signal::Signal;
use crate::process::Process;
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

	// Checking if the thread to kill is the current
	if proc.tid == tid {
		proc.kill(&signal, false);
	} else {
		// Getting the thread
		let thread_mutex = Process::get_by_tid(tid).ok_or(errno!(ESRCH))?;
		let mut thread = thread_mutex.lock();

		// Checking permissions
		if thread.can_kill(proc.uid) || thread.can_kill(proc.euid) {
			return Err(errno!(EPERM));
		}

		thread.kill(&signal, false);
	}

	Ok(0)
}
