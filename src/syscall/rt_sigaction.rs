//! The `rt_sigaction` system call sets the action for a signal.

use core::mem::size_of;
use crate::errno::Errno;
use crate::process::Process;
use crate::process::regs::Regs;
use crate::process::signal::SigAction;
use crate::process::signal::SignalHandler;
use crate::process::signal;

/// The implementation of the `rt_sigaction` syscall.
pub fn rt_sigaction(regs: &Regs) -> Result<i32, Errno> {
	let signum = regs.ebx as i32;
	let act = regs.ecx as *const SigAction;
	let oldact = regs.edx as *mut SigAction;

	if signum as usize >= signal::SIGNALS_COUNT {
		return Err(errno!(EINVAL));
	}

	let mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock();
	let proc = guard.get_mut();

	// Checking access to the given pointers
	if !act.is_null() {
		if !proc.get_mem_space().unwrap().can_access(act as _, size_of::<SigAction>(), true,
			false) {
			return Err(errno!(EFAULT));
		}
	}
	if !oldact.is_null() {
		if !proc.get_mem_space().unwrap().can_access(oldact as _, size_of::<SigAction>(), true,
			true) {
			return Err(errno!(EFAULT));
		}
	}

	// Save the old structure
	if !oldact.is_null() {
		let action = proc.get_signal_handler(signum).get_action();
		unsafe { // Safe because access is checked before
			*oldact = action;
		}
	}

	// Set the new structure
	if !act.is_null() {
		unsafe { // Safe because access is checked before
			proc.set_signal_handler(signum, SignalHandler::Handler(*act));
		}
	}

	Ok(0)
}
