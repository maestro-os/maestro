//! The `sigreturn` system call is used whenever the process finished executing a signal handler.
//! The system call restores the previous state of the process to allow normal execution.

use crate::process::Process;
use crate::process::regs::Regs;

/// The implementation of the `sigreturn` syscall.
pub fn sigreturn(_regs: &Regs) -> ! {
	cli!();

	let mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock();
	let proc = guard.get_mut();

	// Restores the state of the process before the signal handler
	proc.signal_restore();

	let regs = proc.get_regs().clone();
	drop(guard);

	unsafe {
		regs.switch(true);
	}
}
