//! The `rename` system call renames a file.

use crate::errno::Errno;
use crate::file;
use crate::file::path::Path;
use crate::file::vfs;
use crate::file::FileType;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::Process;
use macros::syscall;

// TODO implementation probably can be merged with `renameat2`

#[syscall]
pub fn rename(oldpath: SyscallString, newpath: SyscallString) -> Result<i32, Errno> {
	let (old_path, mut new_parent_path, ap) = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();

		let oldpath = oldpath
			.get(&mem_space_guard)?
			.ok_or_else(|| errno!(EFAULT))?;
		let old_path = Path::from_str(oldpath, true)?;

		let newpath = newpath
			.get(&mem_space_guard)?
			.ok_or_else(|| errno!(EFAULT))?;
		let new_parent_path = Path::from_str(newpath, true)?;

		(old_path, new_parent_path, proc.access_profile)
	};
	let new_name = new_parent_path.pop().ok_or_else(|| errno!(ENOENT))?;

	let old_mutex = vfs::get_file_from_path(&old_path, &ap, false)?;
	let mut old = old_mutex.lock();

	let new_parent_mutex = vfs::get_file_from_path(&new_parent_path, &ap, true)?;
	let mut new_parent = new_parent_mutex.lock();

	// TODO Check permissions if sticky bit is set

	if new_parent.get_location().get_mountpoint_id() == old.get_location().get_mountpoint_id() {
		// Old and new are both on the same filesystem

		// TODO On fail, undo

		// Create link at new location
		// The `..` entry is already updated by the file system since having the same
		// directory in several locations is not allowed
		vfs::create_link(&mut old, &new_parent, &new_name, &ap)?;

		if old.get_type() != FileType::Directory {
			vfs::remove_file(&mut old, &ap)?;
		}
	} else {
		// Old and new are on different filesystems.

		// TODO On fail, undo

		file::util::copy_file(&mut old, &mut new_parent, new_name)?;
		file::util::remove_recursive(&mut old, &ap)?;
	}

	Ok(0)
}
