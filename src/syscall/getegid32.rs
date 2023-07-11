//! The `getegid32` syscall returns the effective GID of the process's owner.

use crate::errno::Errno;
use crate::process::Process;
use macros::syscall;

#[syscall]
pub fn getegid32() -> Result<i32, Errno> {
	let proc_mutex = Process::current_assert();
	let proc = proc_mutex.lock();

	Ok(proc.egid as _)
}
