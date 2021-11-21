//! The `sigreturn` system call is used whenever the process finished executing a signal handler.
//! The system call restores the previous state of the process to allow normal execution.

use crate::gdt;
use crate::process::Process;
use crate::process::Regs;
use crate::process::scheduler;

/// The implementation of the `sigreturn` syscall.
pub fn sigreturn(_regs: &Regs) -> ! {
	cli!();

	let mut mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock(false);
	let proc = guard.get_mut();

	// Restores the state of the process before the signal handler
	proc.signal_restore();

	let regs = proc.get_regs().clone();
	drop(guard);

	unsafe {
		scheduler::context_switch(&regs,
			(gdt::USER_DATA_OFFSET | 3) as _,
			(gdt::USER_CODE_OFFSET | 3) as _);
	}
}
