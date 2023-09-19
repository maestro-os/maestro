//! The `setgid` syscall sets the GID of the process's owner.

use crate::errno::Errno;
use crate::file::perm::Gid;
use crate::process::Process;
use macros::syscall;

#[syscall]
pub fn setgid(gid: Gid) -> Result<i32, Errno> {
	let proc_mutex = Process::current_assert();
	let mut proc = proc_mutex.lock();

	proc.access_profile.set_gid(gid)?;
	Ok(0)
}
