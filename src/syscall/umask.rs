//! The `umask` syscall is used to set the process's file creation mask.

use crate::errno::Errno;
use crate::file;
use crate::process::Process;
use macros::syscall;

#[syscall]
pub fn umask(mask: file::Mode) -> Result<i32, Errno> {
	let mutex = Process::get_current().unwrap();
	let guard = mutex.lock();
	let proc = guard.get_mut();

	let prev = proc.get_umask();
	proc.set_umask(mask & 0o777);
	Ok(prev as _)
}
