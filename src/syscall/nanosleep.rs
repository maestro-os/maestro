//! The `nanosleep` system call allows to make the current process sleep for a
//! given delay.

use crate::errno::Errno;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::process::Process;
use crate::time;
use crate::time::unit::Timespec32;
use macros::syscall;

// TODO Handle signal interruption (EINTR)

#[syscall]
pub fn nanosleep(req: SyscallPtr<Timespec32>, rem: SyscallPtr<Timespec32>) -> Result<i32, Errno> {
	let clk = b"TODO"; // TODO
	let start_time = time::get_struct::<Timespec32>(clk, true).ok_or(errno!(EINVAL))?;

	let delay = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();

		req.get(&mem_space_guard)?
			.ok_or_else(|| errno!(EFAULT))?
			.clone()
	};

	// Looping until time is elapsed or the process is interrupted by a signal
	loop {
		let curr_time = time::get_struct::<Timespec32>(clk, true).ok_or(errno!(EINVAL))?;

		if curr_time >= start_time + delay {
			break;
		}

		// TODO Allow interruption by signal
		// TODO Make the current process sleep
	}

	// Setting remaining time to zero
	{
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let mem_space = proc.get_mem_space().unwrap();
		let mut mem_space_guard = mem_space.lock();

		let remaining = rem
			.get_mut(&mut mem_space_guard)?
			.ok_or_else(|| errno!(EFAULT))?;
		*remaining = Timespec32::default();
	}

	Ok(0)
}
