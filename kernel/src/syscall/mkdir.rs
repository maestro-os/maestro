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

//! The mkdir system call allows to create a directory.

use crate::{
	file,
	file::{
		path::{Path, PathBuf},
		vfs,
		vfs::ResolutionSettings,
		FileType, Stat,
	},
	process::Process,
	syscall::SyscallString,
	time::{
		clock::{current_time, CLOCK_REALTIME},
		unit::TimestampScale,
	},
};
use utils::{
	errno,
	errno::{EResult, Errno},
};

pub fn mkdir(pathname: SyscallString, mode: file::Mode) -> EResult<usize> {
	let (path, mode, rs) = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let mode = mode & !proc.umask;

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();

		// Path to the directory to create
		let path = pathname.get(&mem_space_guard)?.ok_or(errno!(EFAULT))?;
		let path = PathBuf::try_from(path)?;

		let rs = ResolutionSettings::for_process(&proc, true);
		(path, mode, rs)
	};
	let ts = current_time(CLOCK_REALTIME, TimestampScale::Second)?;
	// If the path is not empty, create
	if let Some(name) = path.file_name() {
		// Get parent directory
		let parent_path = path.parent().unwrap_or(Path::root());
		let parent_mutex = vfs::get_file_from_path(parent_path, &rs)?;
		let mut parent = parent_mutex.lock();
		// Create the directory
		vfs::create_file(
			&mut parent,
			name,
			&rs.access_profile,
			Stat {
				file_type: FileType::Directory,
				mode,
				ctime: ts,
				mtime: ts,
				atime: ts,
				..Default::default()
			},
		)?;
	}
	Ok(0)
}
