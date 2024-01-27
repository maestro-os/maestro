//! The `rename` system call renames a file.

use crate::errno::Errno;
use crate::file;
use crate::file::path::PathBuf;
use crate::file::vfs;
use crate::file::vfs::ResolutionSettings;
use crate::file::FileType;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::Process;
use crate::util::container::string::String;
use macros::syscall;

// TODO implementation probably can be merged with `renameat2`

#[syscall]
pub fn rename(oldpath: SyscallString, newpath: SyscallString) -> Result<i32, Errno> {
	let (old_path, mut new_path, mut rs) = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();

		let oldpath = oldpath
			.get(&mem_space_guard)?
			.ok_or_else(|| errno!(EFAULT))?;
		let old_path = PathBuf::try_from(oldpath)?;

		let newpath = newpath
			.get(&mem_space_guard)?
			.ok_or_else(|| errno!(EFAULT))?;
		let new_path = PathBuf::try_from(newpath)?;

		let rs = ResolutionSettings::for_process(&proc, false);
		(old_path, new_path, rs)
	};

	let old_mutex = vfs::get_file_from_path(&old_path, &rs)?;
	let mut old = old_mutex.lock();
	// Cannot rename mountpoint
	if old.is_mountpoint() {
		return Err(errno!(EBUSY));
	}

	let new_parent_path = new_path.parent().ok_or_else(|| errno!(ENOENT))?;
	let new_parent_mutex = vfs::get_file_from_path(
		&new_parent_path,
		&ResolutionSettings {
			follow_link: true,
			..rs
		},
	)?;
	let mut new_parent = new_parent_mutex.lock();
	let new_name = new_path.file_name().ok_or_else(|| errno!(ENOENT))?;

	// TODO Check permissions if sticky bit is set

	if new_parent.get_location() == old.get_location() {
		// Old and new are both on the same filesystem

		// TODO On fail, undo

		// Create link at new location
		// The `..` entry is already updated by the file system since having the same
		// directory in several locations is not allowed
		vfs::create_link(&new_parent, &new_name, &mut old, &rs.access_profile)?;

		if old.get_type() != FileType::Directory {
			vfs::remove_file(&mut old, &rs.access_profile)?;
		}
	} else {
		// Old and new are on different filesystems.

		// TODO On fail, undo

		file::util::copy_file(&mut old, &mut new_parent, String::try_from(new_name)?, &rs)?;
		file::util::remove_recursive(&mut old, &rs)?;
	}

	Ok(0)
}
