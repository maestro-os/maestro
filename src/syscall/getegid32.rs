//! The `getegid32` syscall returns the effective GID of the process's owner.

use crate::errno::Errno;
use crate::process::Process;
use macros::syscall;

#[syscall]
pub fn getegid32() -> Result<i32, Errno> {
	let proc_mutex = Process::get_current().unwrap();
	let proc = proc_mutex.lock();

	Ok(proc.get_egid() as _)
}
