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

//! The `symlink` syscall allows to create a symbolic link.

use crate::{
	file::{
		path::{Path, PathBuf},
		vfs,
		vfs::ResolutionSettings,
	},
	limits,
	process::{mem_space::ptr::SyscallString, Process},
};
use macros::syscall;
use utils::{errno, errno::Errno};

#[syscall]
pub fn symlink(target: SyscallString, linkpath: SyscallString) -> Result<i32, Errno> {
	let (target, linkpath, rs) = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let rs = ResolutionSettings::for_process(&proc, true);

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();

		let target_slice = target
			.get(&mem_space_guard)?
			.ok_or_else(|| errno!(EFAULT))?;
		if target_slice.len() > limits::SYMLINK_MAX {
			return Err(errno!(ENAMETOOLONG));
		}
		let target = PathBuf::try_from(target_slice)?;

		let linkpath = linkpath
			.get(&mem_space_guard)?
			.ok_or_else(|| errno!(EFAULT))?;
		let linkpath = PathBuf::try_from(linkpath)?;

		(target, linkpath, rs)
	};

	// Get the path of the parent directory
	let parent_path = linkpath.parent().unwrap_or(Path::root());
	// The file's basename
	let name = linkpath.file_name().ok_or_else(|| errno!(ENOENT))?;

	// The parent directory
	let parent_mutex = vfs::get_file_from_path(parent_path, &rs)?;
	let mut parent = parent_mutex.lock();

	vfs::create_file(
		&mut parent,
		name.try_into()?,
		&rs.access_profile,
		0o777,
		FileContent::Link(target),
	)?;

	Ok(0)
}
