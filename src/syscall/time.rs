//! The `time` syscall allows to retrieve the number of seconds elapsed since the UNIX Epoch.

use core::mem::size_of;
use crate::errno::Errno;
use crate::errno;
use crate::process::Process;
use crate::process::Regs;
use crate::time;

// TODO Watch for timestamp overflow

/// The implementation of the `time` syscall.
pub fn time(regs: &Regs) -> Result<i32, Errno> {
	let tloc = regs.ebx as *mut u32;

	if !tloc.is_null() {
		let mutex = Process::get_current().unwrap();
		let mut guard = mutex.lock();
		let proc = guard.get_mut();

		if !proc.get_mem_space().unwrap().can_access(tloc as _, size_of::<u32>(), true, true) {
			return Err(errno::EFAULT);
		}
	}

	// Getting the current timestamp
	let time = time::get().unwrap_or(0);

	// Writing the timestamp to the given location, if not null
	if !tloc.is_null() {
		unsafe { // Safe because the access is checked before
			*tloc = time;
		}
	}

	Ok(time as _)
}
