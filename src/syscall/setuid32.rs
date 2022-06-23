//! The `setuid32` syscall sets the UID of the process's owner.

use crate::file::ROOT_UID;
use crate::errno::Errno;
use crate::file::Uid;
use crate::process::Process;
use crate::process::regs::Regs;

/// The implementation of the `setuid32` syscall.
pub fn setuid32(regs: &Regs) -> Result<i32, Errno> {
	let uid = regs.ebx as Uid;

	let mutex = Process::get_current().unwrap();
	let guard = mutex.lock();
	let proc = guard.get_mut();

	// TODO Implement correctly
	if proc.get_uid() == ROOT_UID && proc.get_euid() == ROOT_UID {
		proc.set_uid(uid);
		proc.set_euid(uid);
		proc.set_suid(uid);

		Ok(0)
	} else {
		Err(errno!(EPERM))
	}
}
