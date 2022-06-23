//! The `setgid32` syscall sets the GID of the process's owner.

use crate::file::ROOT_GID;
use crate::errno::Errno;
use crate::file::Gid;
use crate::process::Process;
use crate::process::regs::Regs;

/// The implementation of the `setgid32` syscall.
pub fn setgid32(regs: &Regs) -> Result<i32, Errno> {
	let gid = regs.ebx as Gid;

	let mutex = Process::get_current().unwrap();
	let guard = mutex.lock();
	let proc = guard.get_mut();

	// TODO Implement correctly
	if proc.get_gid() == ROOT_GID && proc.get_egid() == ROOT_GID {
		proc.set_gid(gid);
		proc.set_egid(gid);
		proc.set_sgid(gid);

		Ok(0)
	} else {
		Err(errno!(EPERM))
	}
}
