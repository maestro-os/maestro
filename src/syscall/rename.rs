/*
 * Copyright 2024 Luc Lenôtre
 *
 * This file is part of Maestro.
 *
 * Maestro is free software: you can redistribute it and/or modify it under the
 * terms of the GNU General Public License as published by the Free Software
 * Foundation, either version 3 of the License, or (at your option) any later
 * version.
 *
 * Maestro is distributed in the hope that it will be useful, but WITHOUT ANY
 * WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR
 * A PARTICULAR PURPOSE. See the GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along with
 * Maestro. If not, see <https://www.gnu.org/licenses/>.
 */

//! The `rename` system call renames a file.

use crate::errno::Errno;
use crate::file;
use crate::file::path::PathBuf;
use crate::file::vfs;
use crate::file::FileType;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::Process;
use crate::util::container::string::String;
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
		let old_path = PathBuf::try_from(oldpath)?;

		let newpath = newpath
			.get(&mem_space_guard)?
			.ok_or_else(|| errno!(EFAULT))?;
		let new_parent_path = PathBuf::try_from(newpath)?;

		(old_path, new_parent_path, proc.access_profile)
	};
	let new_name = new_parent_path.file_name().ok_or_else(|| errno!(ENOENT))?;

	let old_mutex = vfs::get_file_from_path(&old_path, &ap, false)?;
	let mut old = old_mutex.lock();

	let new_parent_mutex = vfs::get_file_from_path(&new_parent_path, &ap, true)?;
	let mut new_parent = new_parent_mutex.lock();

	// TODO Check permissions if sticky bit is set

	if new_parent.get_location() == old.get_location() {
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

		file::util::copy_file(&mut old, &mut new_parent, String::try_from(new_name)?)?;
		file::util::remove_recursive(&mut old, &ap)?;
	}

	Ok(0)
}
