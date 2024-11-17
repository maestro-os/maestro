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

//! The `statfs` system call returns information about a mounted file system.

use crate::{
	file::{fs::Statfs, vfs, vfs::ResolutionSettings},
	process::{
		mem_space::copy::{SyscallPtr, SyscallString},
		Process,
	},
	syscall::Args,
};
use utils::{
	collections::path::PathBuf,
	errno,
	errno::{EResult, Errno},
};

pub(super) fn do_statfs(
	path: SyscallString,
	buf: SyscallPtr<Statfs>,
	rs: ResolutionSettings,
) -> EResult<usize> {
	let rs = ResolutionSettings {
		follow_link: false,
		..rs
	};
	let path = path.copy_from_user()?.ok_or_else(|| errno!(EFAULT))?;
	let path = PathBuf::try_from(path)?;
	let stat = vfs::get_file_from_path(&path, &rs)?
		.node()
		.location
		.get_mountpoint()
		// Unwrapping will not fail since the file is accessed from path
		.unwrap()
		.fs
		.get_stat()?;
	// Write structure to userspace
	buf.copy_to_user(&stat)?;
	Ok(0)
}

pub fn statfs(
	Args((path, buf)): Args<(SyscallString, SyscallPtr<Statfs>)>,
	rs: ResolutionSettings,
) -> EResult<usize> {
	do_statfs(path, buf, rs)
}
