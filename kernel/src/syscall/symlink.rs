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
	file::{vfs, vfs::ResolutionSettings, FileType, Stat},
	process::{mem_space::copy::SyscallString, Process},
	syscall::Args,
	time::{
		clock::{current_time, CLOCK_REALTIME},
		unit::TimestampScale,
	},
};
use utils::{
	collections::path::{Path, PathBuf},
	errno,
	errno::{EResult, Errno},
	limits::SYMLINK_MAX,
};

pub fn symlink(
	Args((target, linkpath)): Args<(SyscallString, SyscallString)>,
	rs: ResolutionSettings,
) -> EResult<usize> {
	let target_slice = target.copy_from_user()?.ok_or_else(|| errno!(EFAULT))?;
	if target_slice.len() > SYMLINK_MAX {
		return Err(errno!(ENAMETOOLONG));
	}
	let target = PathBuf::try_from(target_slice)?;
	let linkpath = linkpath.copy_from_user()?.ok_or_else(|| errno!(EFAULT))?;
	let linkpath = PathBuf::try_from(linkpath)?;
	let link_parent = linkpath.parent().unwrap_or(Path::root());
	let link_name = linkpath.file_name().ok_or_else(|| errno!(ENOENT))?;
	// Link's parent
	let parent = vfs::get_file_from_path(link_parent, &rs)?;
	// Create link
	let ts = current_time(CLOCK_REALTIME, TimestampScale::Second)?;
	let file = vfs::create_file(
		parent,
		link_name,
		&rs.access_profile,
		Stat {
			mode: FileType::Link.to_mode() | 0o777,
			ctime: ts,
			mtime: ts,
			atime: ts,
			..Default::default()
		},
	)?;
	// TODO remove file on failure
	file.node()
		.node_ops
		.write_content(&file.node().location, 0, target.as_bytes())?;
	Ok(0)
}
