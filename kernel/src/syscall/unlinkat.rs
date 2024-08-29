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

//! The `unlinkat` syscall allows to unlink a file.
//!
//! If no link remain to the file, the function also removes it.

use super::util::at;
use crate::{
	file::{
		fd::FileDescriptorTable,
		path::PathBuf,
		vfs,
		vfs::{ResolutionSettings, Resolved},
	},
	process::{mem_space::copy::SyscallString, Process},
	syscall::{util::at::AT_EMPTY_PATH, Args},
};
use core::ffi::c_int;
use utils::{
	errno,
	errno::{EResult, Errno},
	lock::Mutex,
	ptr::arc::Arc,
};

pub fn unlinkat(
	Args((dirfd, pathname, flags)): Args<(c_int, SyscallString, c_int)>,
	rs: ResolutionSettings,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	let pathname = pathname.copy_from_user()?.ok_or_else(|| errno!(EFAULT))?;
	let path = PathBuf::try_from(pathname)?;
	let parent_path = path.parent().ok_or_else(|| errno!(ENOENT))?;
	let rs = ResolutionSettings {
		follow_link: false,
		..rs
	};
	// AT_EMPTY_PATH is required in case the path has only one component
	let resolved = at::get_file(
		&fds.lock(),
		rs.clone(),
		dirfd,
		parent_path,
		flags | AT_EMPTY_PATH,
	)?;
	let Resolved::Found(parent) = resolved else {
		return Err(errno!(ENOENT));
	};
	let name = path.file_name().ok_or_else(|| errno!(ENOENT))?;
	vfs::unlink(parent, name, &rs.access_profile)?;
	Ok(0)
}
