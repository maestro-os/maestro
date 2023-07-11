//! The `time` syscall allows to retrieve the number of seconds elapsed since
//! the UNIX Epoch.

use crate::errno::Errno;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::process::Process;
use crate::time::clock;
use crate::time::clock::CLOCK_MONOTONIC;
use crate::time::unit::TimestampScale;
use macros::syscall;

// TODO Watch for timestamp overflow

#[syscall]
pub fn time(tloc: SyscallPtr<u32>) -> Result<i32, Errno> {
	let proc_mutex = Process::current_assert();
	let proc = proc_mutex.lock();

	let mem_space = proc.get_mem_space().unwrap();
	let mut mem_space_guard = mem_space.lock();

	// Getting the current timestamp
	let time = clock::current_time(CLOCK_MONOTONIC, TimestampScale::Second)?;

	// Writing the timestamp to the given location, if not null
	if let Some(tloc) = tloc.get_mut(&mut mem_space_guard)? {
		*tloc = time as _;
	}

	Ok(time as _)
}
