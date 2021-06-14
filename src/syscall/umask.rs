//! The `umask` syscall is used to set the process's file creation mask.

use crate::errno::Errno;
use crate::process::Process;
use crate::util;

/// The implementation of the `umask` syscall.
pub fn umask(proc: &mut Process, regs: &util::Regs) -> Result<i32, Errno> {
	let mask = regs.ebx as u16;

	let prev = proc.get_umask();
	proc.set_umask(mask & 0o777);
	Ok(prev as _)
}
