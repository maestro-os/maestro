//! The _exit syscall allows to terminate the current process with the given
//! status code.

use crate::errno::Errno;
use crate::process::Process;
use core::arch::asm;
use core::ffi::c_int;
use macros::syscall;

/// Exits the current process.
/// `status` is the exit status.
/// `thread_group`: if true, the function exits the whole process group.
pub fn do_exit(status: u32, thread_group: bool) -> ! {
	let (_pid, _tid) = {
		let mutex = Process::get_current().unwrap();
		let guard = mutex.lock();
		let proc = guard.get_mut();

		proc.exit(status, false);

		(proc.get_pid(), proc.get_tid())
	};

	if thread_group {
		// TODO Iterate on every process of thread group `tid`, except the
		// process with pid `pid`
	}

	unsafe {
		// Waiting for the next tick
		asm!("jmp $kernel_loop");
	}

	// This loop is here only to avoid a compilation error
	loop {}
}

#[syscall]
pub fn _exit(status: c_int) -> Result<i32, Errno> {
	do_exit(status as _, false);
}
