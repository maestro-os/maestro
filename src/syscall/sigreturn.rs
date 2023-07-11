//! The `sigreturn` system call is used whenever the process finished executing
//! a signal handler.
//!
//! The system call restores the previous state of the process
//! to allow normal execution.

use crate::errno::Errno;
use crate::process::Process;
use macros::syscall;

#[syscall]
pub fn sigreturn() -> Result<i32, Errno> {
	cli!();

	let regs = {
		let proc_mutex = Process::current_assert();
		let mut proc = proc_mutex.lock();

		// Restores the state of the process before the signal handler
		proc.signal_restore();

		proc.regs.clone()
	};

	unsafe {
		regs.switch(true);
	}
}
