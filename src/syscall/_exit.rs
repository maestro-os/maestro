//! The _exit syscall allows to terminate the current process with the given
//! status code.

use crate::errno::Errno;
use crate::process::scheduler;
use crate::process::Process;
use core::ffi::c_int;
use macros::syscall;

/// Exits the current process.
///
/// Arguments:
/// - `status` is the exit status.
/// - `thread_group`: if `true`, the function exits the whole process group.
pub fn do_exit(status: u32, thread_group: bool) -> ! {
	let (_pid, _tid) = {
		let proc_mutex = Process::current_assert();
		let mut proc = proc_mutex.lock();

		proc.exit(status, false);

		(proc.pid, proc.tid)
	};

	if thread_group {
		// TODO Iterate on every process of thread group `tid`, except the
		// process with pid `pid`
	}

	scheduler::end_tick();
	// Cannot resume since the process is now a zombie
	unreachable!();
}

#[syscall]
pub fn _exit(status: c_int) -> Result<i32, Errno> {
	do_exit(status as _, false);
}
