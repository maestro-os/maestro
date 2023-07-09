//! The `getegid` syscall returns the effective GID of the process's owner.

use crate::errno::Errno;
use crate::process::regs::Regs;
use crate::process::Process;

pub fn getegid(_: &Regs) -> Result<i32, Errno> {
	let proc_mutex = Process::current_assert();
	let proc = proc_mutex.lock();

	Ok(proc.egid as _)
}
