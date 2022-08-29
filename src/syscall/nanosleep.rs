//! The `nanosleep` system call allows to make the current process sleep for a given delay.

use crate::errno::Errno;
use crate::process::Process;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::process::regs::Regs;
use crate::time::unit::Timespec;
use crate::time;

// TODO Handle signal interruption (EINTR)

/// The implementation of the `nanosleep` syscall.
pub fn nanosleep(regs: &Regs) -> Result<i32, Errno> {
	let req: SyscallPtr<Timespec> = (regs.ebx as usize).into();
	let _rem: SyscallPtr<Timespec> = (regs.ecx as usize).into();

	let clk = b"TODO"; // TODO
	let start_time = time::get_struct::<Timespec>(clk, true).ok_or(errno!(EINVAL))?;

	let delay = {
		let proc_mutex = Process::get_current().unwrap();
		let proc_guard = proc_mutex.lock();
		let proc = proc_guard.get();

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();

		req.get_mut(&mem_space_guard)?.ok_or_else(|| errno!(EFAULT))?.clone()
	};

	// Looping until time is elapsed or the process is interrupted by a signal
	loop {
		let curr_time = time::get_struct::<Timespec>(clk, true).ok_or(errno!(EINVAL))?;

		if curr_time >= start_time + delay {
			break;
		}

		// TODO Make the current process sleep
	}

	Ok(0)
}
