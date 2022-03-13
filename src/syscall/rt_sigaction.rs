//! The `rt_sigaction` system call sets the action for a signal.

use crate::errno::Errno;
use crate::process::Process;
use crate::process::Regs;
use crate::process::signal::SigAction;

/// The implementation of the `rt_sigaction` syscall.
pub fn rt_sigaction(regs: &Regs) -> Result<i32, Errno> {
    let _signum = regs.ebx as i32;
    let act = regs.ecx as *const SigAction;
    let oldact = regs.edx as *mut SigAction;

    let mutex = Process::get_current().unwrap();
    let mut guard = mutex.lock();
    let _proc = guard.get_mut();

	// Checking access to the given pointers
	if !act.is_null() {
		// TODO Check access
	}
	if !oldact.is_null() {
		// TODO Check access
	}

	// Save the old structure
	if !oldact.is_null() {
		// TODO Writes the old action
		todo!();
	}

	// Set the new structure
	if !act.is_null() {
		// TODO Sets the new action
		todo!();
	}

	Ok(0)
}
