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

//! The `mknod` system call allows to create a new node on a filesystem.

use crate::{
	device::id,
	file,
	file::{vfs, vfs::ResolutionSettings, FileType, Stat},
	process::{mem_space::copy::SyscallString, Process},
	syscall::{Args, Umask},
	time::{
		clock::{current_time, CLOCK_REALTIME},
		unit::TimestampScale,
	},
};
use utils::{
	collections::path::{Path, PathBuf},
	errno,
	errno::{EResult, Errno},
};

pub fn mknod(
	Args((pathname, mode, dev)): Args<(SyscallString, file::Mode, u64)>,
	umask: Umask,
	rs: ResolutionSettings,
) -> EResult<usize> {
	let path = pathname.copy_from_user()?.ok_or(errno!(EFAULT))?;
	let path = PathBuf::try_from(path)?;
	let parent_path = path.parent().unwrap_or(Path::root());
	// File name
	let Some(name) = path.file_name() else {
		return Err(errno!(EEXIST));
	};
	// Check file type and permissions
	let mode = mode & !umask.0;
	let file_type = FileType::from_mode(mode).ok_or(errno!(EPERM))?;
	let privileged = rs.access_profile.is_privileged();
	match (file_type, privileged) {
		(FileType::Regular | FileType::Fifo | FileType::Socket, _) => {}
		(FileType::BlockDevice | FileType::CharDevice, true) => {}
		(_, false) => return Err(errno!(EPERM)),
		(_, true) => return Err(errno!(EINVAL)),
	}
	// Create file
	let ts = current_time(CLOCK_REALTIME, TimestampScale::Second)?;
	let parent = vfs::get_file_from_path(parent_path, &rs)?;
	vfs::create_file(
		parent,
		name,
		&rs.access_profile,
		Stat {
			mode,
			dev_major: id::major(dev),
			dev_minor: id::minor(dev),
			ctime: ts,
			mtime: ts,
			atime: ts,
			..Default::default()
		},
	)?;
	Ok(0)
}
