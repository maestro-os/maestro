//! The `getgid` syscall returns the GID of the process's owner.

use crate::errno::Errno;
use crate::process::Process;
use macros::syscall;

#[syscall]
pub fn getgid() -> Result<i32, Errno> {
	let proc_mutex = Process::current_assert();
	let proc = proc_mutex.lock();

	Ok(proc.gid as _)
}
