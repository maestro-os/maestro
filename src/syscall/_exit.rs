//! The _exit syscall allows to terminate the current process with the given status code.

use crate::process::Process;
use crate::util::lock::mutex::TMutex;
use crate::util;

/// The implementation of the `write` syscall.
pub fn _exit(proc: &mut Process, regs: &util::Regs) -> ! {
	proc.exit(regs.eax);

	unsafe {
		// Unlocking the process because the system call is not returning
		Process::get_current().unwrap().unlock();
		// Waiting for the next tick
		asm!("jmp $kernel_loop");
	}

	// This loop is here only to avoid a compilation error
	loop {}
}
