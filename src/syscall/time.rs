//! The `time` syscall allows to retrieve the number of seconds elapsed since
//! the UNIX Epoch.

use crate::errno::Errno;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::process::Process;
use crate::time;
use crate::time::unit::TimestampScale;
use macros::syscall;

// TODO Watch for timestamp overflow

#[syscall]
pub fn time(tloc: SyscallPtr<u32>) -> Result<i32, Errno> {
	let mutex = Process::get_current().unwrap();
	let guard = mutex.lock();
	let proc = guard.get_mut();

	let mem_space = proc.get_mem_space().unwrap();
	let mem_space_guard = mem_space.lock();

	// Getting the current timestamp
	let time = time::get(TimestampScale::Second, false).unwrap_or(0);

	// Writing the timestamp to the given location, if not null
	if let Some(tloc) = tloc.get_mut(&mem_space_guard)? {
		*tloc = time as _;
	}

	Ok(time as _)
}
