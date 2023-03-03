//! This `linkat` syscall creates a new hard link to a file.

use super::access;
use crate::errno::Errno;
use crate::file::vfs;
use crate::file::FileType;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::Process;
use crate::types::c_int;
use macros::syscall;

#[syscall]
pub fn linkat(
	olddirfd: c_int,
	oldpath: SyscallString,
	newdirfd: c_int,
	newpath: SyscallString,
	flags: c_int,
) -> Result<i32, Errno> {
	let follow_links = flags & access::AT_SYMLINK_NOFOLLOW == 0;

	let (old_mutex, new_parent_mutex, new_name, uid, gid) = {
		let proc_mutex = Process::get_current().unwrap();
		let proc_guard = proc_mutex.lock();
		let proc = proc_guard.get();

		let euid = proc.get_euid();
		let egid = proc.get_egid();

		let mem_space = proc.get_mem_space().clone().unwrap();
		let mem_space_guard = mem_space.lock();

		let oldpath = oldpath
			.get(&mem_space_guard)?
			.ok_or_else(|| errno!(EFAULT))?;
		let old = super::util::get_file_at(proc_guard, follow_links, olddirfd, oldpath, flags)?;

		let proc_guard = proc_mutex.lock();
		let newpath = newpath
			.get(&mem_space_guard)?
			.ok_or_else(|| errno!(EFAULT))?;
		let (new_parent, new_name) =
			super::util::get_parent_at_with_name(proc_guard, follow_links, newdirfd, newpath)?;

		(old, new_parent, new_name, euid, egid)
	};

	let old_guard = old_mutex.lock();
	let old = old_guard.get_mut();

	if old.get_type() == FileType::Directory {
		return Err(errno!(EISDIR));
	}

	let new_parent_guard = new_parent_mutex.lock();
	let new_parent = new_parent_guard.get_mut();

	let vfs_mutex = vfs::get();
	let vfs_guard = vfs_mutex.lock();
	let vfs = vfs_guard.get_mut().as_mut().unwrap();

	vfs.create_link(old, new_parent, new_name, uid, gid)?;
	Ok(0)
}
