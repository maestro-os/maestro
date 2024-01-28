//! The `setgid32` syscall sets the GID of the process's owner.

use crate::{errno::Errno, file::perm::Gid, process::Process};
use macros::syscall;

#[syscall]
pub fn setgid32(gid: Gid) -> Result<i32, Errno> {
	let proc_mutex = Process::current_assert();
	let mut proc = proc_mutex.lock();

	proc.access_profile.set_gid(gid)?;
	Ok(0)
}
