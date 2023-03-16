//! The `setuid32` syscall sets the UID of the process's owner.

use crate::errno::Errno;
use crate::file::Uid;
use crate::file::ROOT_UID;
use crate::process::Process;
use macros::syscall;

#[syscall]
pub fn setuid32(uid: Uid) -> Result<i32, Errno> {
	let proc_mutex = Process::get_current().unwrap();
	let proc = proc_mutex.lock();

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
