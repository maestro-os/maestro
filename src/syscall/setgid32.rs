//! The `setgid32` syscall sets the GID of the process's owner.

use crate::errno::Errno;
use crate::file::Gid;
use crate::file::ROOT_GID;
use crate::process::Process;
use macros::syscall;

/// The implementation of the `setgid32` syscall.
#[syscall]
pub fn setgid32(gid: Gid) -> Result<i32, Errno> {
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
