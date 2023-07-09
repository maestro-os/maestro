//! The `setgid32` syscall sets the GID of the process's owner.

use crate::errno::Errno;
use crate::file::Gid;
use crate::file::ROOT_GID;
use crate::process::Process;
use macros::syscall;

#[syscall]
pub fn setgid32(gid: Gid) -> Result<i32, Errno> {
	let proc_mutex = Process::current_assert();
	let mut proc = proc_mutex.lock();

	// TODO Implement correctly
	if proc.gid == ROOT_GID && proc.egid == ROOT_GID {
		proc.gid = gid;
		proc.egid = gid;
		proc.sgid = gid;

		Ok(0)
	} else {
		Err(errno!(EPERM))
	}
}
