//! The _exit syscall allows to terminate the current process with the given status code.

use core::arch::asm;
use crate::errno::Errno;
use crate::process::Process;
use crate::process::regs::Regs;

/// TODO doc
/// `status` is the exit status.
/// `thread_group`: if true, the function exits the whole process group.
pub fn do_exit(status: u32, thread_group: bool) -> ! {
	let (_pid, _tid) = {
		let mutex = Process::get_current().unwrap();
		let mut guard = mutex.lock();
		let proc = guard.get_mut();

		proc.exit(status);

		(proc.get_pid(), proc.get_tid())
	};

	if thread_group {
		// TODO Iterate on every process of thread group `tid`, except the process with pid `pid`
	}

	unsafe {
		// Waiting for the next tick
		asm!("jmp $kernel_loop");
	}

	// This loop is here only to avoid a compilation error
	loop {}
}

/// The implementation of the `_exit` syscall.
pub fn _exit(regs: &Regs) -> Result<i32, Errno> {
	let status = regs.ebx as i32;

	do_exit(status as _, false);
}
