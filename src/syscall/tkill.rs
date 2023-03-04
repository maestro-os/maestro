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
	let signal = Signal::from_id(sig as _)?;

	let mutex = Process::get_current().unwrap();
	let guard = mutex.lock();
	let proc = guard.get_mut();

	// Checking if the thread to kill is the current
	if proc.get_tid() == tid {
		proc.kill(&signal, false);
	} else {
		// Getting the thread
		let thread_mutex = Process::get_by_tid(tid).ok_or(errno!(ESRCH))?;
		let thread_guard = thread_mutex.lock();
		let thread = thread_guard.get_mut();

		// Checking permissions
		if thread.can_kill(proc.get_uid()) || thread.can_kill(proc.get_euid()) {
			return Err(errno!(EPERM));
		}

		thread.kill(&signal, false);
	}

	Ok(0)
}
