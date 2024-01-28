//! The `setuid32` syscall sets the UID of the process's owner.

use crate::{errno::Errno, file::perm::Uid, process::Process};
use macros::syscall;

#[syscall]
pub fn setuid32(uid: Uid) -> Result<i32, Errno> {
	let proc_mutex = Process::current_assert();
	let mut proc = proc_mutex.lock();

	proc.access_profile.set_uid(uid)?;
	Ok(0)
}
