//! The `rt_sigaction` system call sets the action for a signal.

use crate::errno::Errno;
use crate::process::Process;
use crate::process::Regs;
use crate::process::signal::SigAction;
use crate::process::signal::SignalHandler;

/// The implementation of the `rt_sigaction` syscall.
pub fn rt_sigaction(regs: &Regs) -> Result<i32, Errno> {
	let signum = regs.ebx as i32;
	let act = regs.ecx as *const SigAction;
	let oldact = regs.edx as *mut SigAction;

	let mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock();
	let proc = guard.get_mut();

	// Checking access to the given pointers
	if !act.is_null() {
		// TODO Check access
	}
	if !oldact.is_null() {
		// TODO Check access
	}

	// Save the old structure
	if !oldact.is_null() {
		match proc.get_signal_handler(signum) {
			SignalHandler::Action(action) => unsafe { // Safe because access is checked before
				*oldact = action;
			},

			_ => {
				// TODO Figure out how to handle this case
				todo!();
			},
		}
	}

	// Set the new structure
	if !act.is_null() {
		unsafe { // Safe because access is checked before
			proc.set_signal_handler(signum, SignalHandler::Action(*act));
		}
	}

	Ok(0)
}
