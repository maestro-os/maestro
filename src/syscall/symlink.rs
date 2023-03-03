//! The `symlink` syscall allows to create a symbolic link.

use crate::errno::Errno;
use crate::file::FileContent;
use crate::file::path::Path;
use crate::file::vfs;
use crate::limits;
use crate::process::Process;
use crate::process::mem_space::ptr::SyscallString;
use crate::util::container::string::String;
use macros::syscall;

/// The implementation of the `symlink` syscall.
#[syscall]
pub fn symlink(
	target: SyscallString,
	linkpath: SyscallString,
) -> Result<i32, Errno> {
	let (uid, gid, target, linkpath) = {
		let mutex = Process::get_current().unwrap();
		let guard = mutex.lock();
		let proc = guard.get_mut();

		let uid = proc.get_euid();
		let gid = proc.get_egid();

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();

		let target_slice = target
			.get(&mem_space_guard)?
			.ok_or_else(|| errno!(EFAULT))?;
		if target_slice.len() > limits::SYMLINK_MAX {
			return Err(errno!(ENAMETOOLONG));
		}
		let target = String::from(target_slice)?;

		let linkpath = linkpath
			.get(&mem_space_guard)?
			.ok_or_else(|| errno!(EFAULT))?;
		let linkpath = Path::from_str(linkpath, true)?;

		(uid, gid, target, linkpath)
	};

	let vfs_mutex = vfs::get();
	let vfs_guard = vfs_mutex.lock();
	let vfs = vfs_guard.get_mut().as_mut().unwrap();

	// Getting the path of the parent directory
	let mut parent_path = linkpath;
	// The file's basename
	let name = parent_path.pop().ok_or_else(|| errno!(ENOENT))?;

	// The parent directory
	let parent_mutex = vfs.get_file_from_path(&parent_path, uid, gid, true)?;
	let parent_guard = parent_mutex.lock();
	let parent = parent_guard.get_mut();

	vfs.create_file(parent, name, uid, gid, 0o777, FileContent::Link(target))?;

	Ok(0)
}
