//! The `renameat2` allows to rename a file.

use super::util::at;
use crate::errno::Errno;
use crate::file;
use crate::file::path::PathBuf;
use crate::file::vfs;
use crate::file::vfs::{ResolutionSettings, Resolved};
use crate::file::FileType;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::Process;
use core::ffi::c_int;
use macros::syscall;

/// Flag: Don't replace new path if it exists. Return an error instead.
const RENAME_NOREPLACE: c_int = 1;
/// Flag: Exchanges old and new paths atomically.
const RENAME_EXCHANGE: c_int = 2;

// TODO implement flags

#[syscall]
pub fn renameat2(
	olddirfd: c_int,
	oldpath: SyscallString,
	newdirfd: c_int,
	newpath: SyscallString,
	_flags: c_int,
) -> Result<i32, Errno> {
	let (fds_mutex, oldpath, newpath, rs) = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let rs = ResolutionSettings::for_process(&*proc, false);

		let mem_space = proc.get_mem_space().unwrap().clone();
		let mem_space_guard = mem_space.lock();

		let fds_mutex = proc.file_descriptors.clone().unwrap();

		let oldpath = oldpath
			.get(&mem_space_guard)?
			.ok_or_else(|| errno!(EFAULT))?;
		let oldpath = PathBuf::try_from(oldpath)?;

		let newpath = newpath
			.get(&mem_space_guard)?
			.ok_or_else(|| errno!(EFAULT))?;
		let newpath = PathBuf::try_from(newpath)?;

		(fds_mutex, oldpath, newpath, rs)
	};

	let fds = fds_mutex.lock();

	let Resolved::Found(old_mutex) = at::get_file(&fds, rs, olddirfd, &oldpath, 0)? else {
		return Err(errno!(ENOENT));
	};
	let mut old = old_mutex.lock();
	// Cannot rename mountpoint
	if old.is_mountpoint() {
		return Err(errno!(EBUSY));
	}

	// TODO RENAME_NOREPLACE
	let Resolved::Creatable {
		parent: new_parent,
		name: new_name,
	} = at::get_file(&fds, rs, newdirfd, &newpath, 0)?
	else {
		return Err(errno!(EEXIST));
	};
	let mut new_parent = new_parent.lock();

	// TODO Check permissions if sticky bit is set

	if new_parent.get_location().get_mountpoint_id() == old.get_location().get_mountpoint_id() {
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

		file::util::copy_file(&mut old, &mut new_parent, new_name, &rs)?;
		file::util::remove_recursive(&mut old, &rs)?;
	}

	Ok(0)
}
