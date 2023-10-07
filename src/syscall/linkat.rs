//! This `linkat` syscall creates a new hard link to a file.

use super::access;
use crate::errno::Errno;
use crate::file::vfs;
use crate::file::FileType;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::Process;
use core::ffi::c_int;
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

	let (old_mutex, new_parent_mutex, new_name, ap) = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let ap = proc.access_profile;

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();

		let oldpath = oldpath
			.get(&mem_space_guard)?
			.ok_or_else(|| errno!(EFAULT))?;
		let old = super::util::get_file_at(proc, follow_links, olddirfd, oldpath, flags)?;

		let proc = proc_mutex.lock();
		let newpath = newpath
			.get(&mem_space_guard)?
			.ok_or_else(|| errno!(EFAULT))?;
		let (new_parent, new_name) =
			super::util::get_parent_at_with_name(proc, follow_links, newdirfd, newpath)?;

		(old, new_parent, new_name, ap)
	};

	let mut old = old_mutex.lock();
	if matches!(old.get_type(), FileType::Directory) {
		return Err(errno!(EISDIR));
	}
	let mut new_parent = new_parent_mutex.lock();

	vfs::create_link(&mut old, &mut new_parent, &new_name, &ap)?;
	Ok(0)
}
