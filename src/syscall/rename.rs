//! The `rename` system call renames a file.

use crate::errno::Errno;
use crate::file;
use crate::file::path::Path;
use crate::file::vfs;
use crate::file::FileType;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::Process;
use macros::syscall;

#[syscall]
pub fn rename(oldpath: SyscallString, newpath: SyscallString) -> Result<i32, Errno> {
	let vfs = vfs::get();
	let mut vfs = vfs.lock();
	let vfs = vfs.as_mut().unwrap();

	let (uid, gid, old_mutex, new_parent_mutex, new_name) = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let uid = proc.euid;
		let gid = proc.egid;

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();

		let oldpath = oldpath
			.get(&mem_space_guard)?
			.ok_or_else(|| errno!(EFAULT))?;
		let old_path = Path::from_str(oldpath, true)?;
		let old = vfs.get_file_from_path(&old_path, uid, gid, false)?;

		let newpath = newpath
			.get(&mem_space_guard)?
			.ok_or_else(|| errno!(EFAULT))?;
		let mut new_parent_path = Path::from_str(newpath, true)?;
		let new_name = new_parent_path.pop().ok_or_else(|| errno!(ENOENT))?;
		let new_parent = vfs.get_file_from_path(&new_parent_path, uid, gid, true)?;

		(uid, gid, old, new_parent, new_name)
	};

	let mut old = old_mutex.lock();
	let mut new_parent = new_parent_mutex.lock();

	// TODO Check permissions if sticky bit is set

	if new_parent.get_location().get_mountpoint_id() == old.get_location().get_mountpoint_id() {
		// Old and new are both on the same filesystem

		// TODO On fail, undo

		// Create link at new location
		// The `..` entry is already updated by the file system since having the same
		// directory in several locations is not allowed
		vfs.create_link(&mut old, &mut new_parent, &new_name, uid, gid)?;

		if old.get_type() != FileType::Directory {
			vfs.remove_file(&old, uid, gid)?;
		}
	} else {
		// Old and new are on different filesystems.

		// TODO On fail, undo

		file::util::copy_file(vfs, &mut old, &mut new_parent, new_name)?;
		file::util::remove_recursive(vfs, &mut old, uid, gid)?;
	}

	Ok(0)
}
