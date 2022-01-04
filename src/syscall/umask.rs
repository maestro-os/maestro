//! The `umask` syscall is used to set the process's file creation mask.

use crate::errno::Errno;
use crate::file;
use crate::process::Process;
use crate::process::Regs;

/// The implementation of the `umask` syscall.
pub fn umask(regs: &Regs) -> Result<i32, Errno> {
	let mask = regs.ebx as file::Mode;

	let mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock();
	let proc = guard.get_mut();

	let prev = proc.get_umask();
	proc.set_umask(mask & 0o777);
	Ok(prev as _)
}
