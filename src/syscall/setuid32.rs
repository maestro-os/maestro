//! The `setuid32` syscall sets the UID of the process's owner.

use crate::errno::Errno;
use crate::file::Uid;
use crate::file::ROOT_UID;
use crate::process::Process;
use macros::syscall;

#[syscall]
pub fn setuid32(uid: Uid) -> Result<i32, Errno> {
	let proc_mutex = Process::current_assert();
	let mut proc = proc_mutex.lock();

	// TODO Implement correctly
	if proc.uid == ROOT_UID && proc.euid == ROOT_UID {
		proc.uid = uid;
		proc.euid = uid;
		proc.suid = uid;

		Ok(0)
	} else {
		Err(errno!(EPERM))
	}
}
