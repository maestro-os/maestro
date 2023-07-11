//! The `umask` syscall is used to set the process's file creation mask.

use crate::errno::Errno;
use crate::file;
use crate::process::Process;
use macros::syscall;

#[syscall]
pub fn umask(mask: file::Mode) -> Result<i32, Errno> {
	let proc_mutex = Process::current_assert();
	let mut proc = proc_mutex.lock();

	let prev = proc.umask;
	proc.umask = mask & 0o777;

	Ok(prev as _)
}
