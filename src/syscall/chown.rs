//! The `chown` system call changes the owner of a file.

use crate::errno::EResult;
use crate::errno::Errno;
use crate::file::path::Path;
use crate::file::perm::{Gid, Uid};
use crate::file::vfs;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::Process;
use macros::syscall;

/// Performs the `chown` syscall.
pub fn do_chown(
	pathname: SyscallString,
	owner: Uid,
	group: Gid,
	follow_links: bool,
) -> EResult<i32> {
	let (path, ap) = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space = mem_space.lock();

		let path = pathname.get(&*mem_space)?.ok_or_else(|| errno!(EFAULT))?;
		(Path::from_str(path, true)?, proc.access_profile)
	};

	let file_mutex = vfs::get_file_from_path(&path, &ap, follow_links)?;
	let mut file = file_mutex.lock();
	file.set_uid(owner);
	file.set_gid(group);
	// TODO lazy
	file.sync()?;

	Ok(0)
}

#[syscall]
pub fn chown(pathname: SyscallString, owner: Uid, group: Gid) -> EResult<i32> {
	do_chown(pathname, owner, group, true)
}
