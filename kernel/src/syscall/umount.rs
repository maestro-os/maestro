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

//! The `umount` system call allows to unmount a filesystem previously mounted
//! with `mount`.

use crate::{
	file::{
		mountpoint,
		path::{Path, PathBuf},
		vfs,
		vfs::ResolutionSettings,
	},
	process::{mem_space::copy::SyscallString, Process},
	syscall::Args,
};
use utils::{
	errno,
	errno::{EResult, Errno},
};

pub fn umount(Args(target): Args<SyscallString>, rs: ResolutionSettings) -> EResult<usize> {
	// Check permission
	if !rs.access_profile.is_privileged() {
		return Err(errno!(EPERM));
	}
	// Get target directory
	let target_slice = target.copy_from_user()?.ok_or(errno!(EFAULT))?;
	let target_path = PathBuf::try_from(target_slice)?;
	let target_location = vfs::get_file_from_path(&target_path, &rs)?.lock().location;
	// Remove mountpoint
	mountpoint::remove(&target_location)?;
	Ok(0)
}
