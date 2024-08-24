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

//! The `chmod` system call allows change the permissions on a file.

use crate::{
	file,
	file::{fs::StatSet, path::PathBuf, vfs, vfs::ResolutionSettings},
	process::{mem_space::copy::SyscallString, Process},
	syscall::Args,
};
use core::ffi::c_int;
use utils::{
	errno,
	errno::{EResult, Errno},
};

pub fn chmod(
	Args((pathname, mode)): Args<(SyscallString, file::Mode)>,
	rs: ResolutionSettings,
) -> EResult<usize> {
	let path = pathname.copy_from_user()?.ok_or_else(|| errno!(EFAULT))?;
	let path = PathBuf::try_from(path)?;
	// Get file
	let file = vfs::get_file_from_path(&path, &rs)?;
	// Check permissions
	let stat = file.get_stat()?;
	if !rs.access_profile.can_set_file_permissions(&stat) {
		return Err(errno!(EPERM));
	}
	file.node.ops.set_stat(
		&file.node.location,
		StatSet {
			mode: Some(mode & 0o777),
			..Default::default()
		},
	)?;
	Ok(0)
}
