//! The `sigreturn` system call is used whenever the process finished executing
//! a signal handler. The system call restores the previous state of the process
//! to allow normal execution.

use crate::errno::Errno;
use crate::process::Process;
use macros::syscall;

#[syscall]
pub fn sigreturn() -> Result<i32, Errno> {
	cli!();

	let regs = {
		let mutex = Process::get_current().unwrap();
		let guard = mutex.lock();
		let proc = guard.get_mut();

		// Restores the state of the process before the signal handler
		proc.signal_restore();

		proc.get_regs().clone()
	};

	unsafe {
		regs.switch(true);
	}
}
