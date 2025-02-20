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

//! The `fchmodat` system call allows change the permissions on a file.

use super::util::at;
use crate::{
	file,
	file::{
		fd::FileDescriptorTable,
		fs::StatSet,
		vfs,
		vfs::{ResolutionSettings, Resolved},
	},
	process::{mem_space::copy::SyscallString, Process},
	sync::mutex::Mutex,
	syscall::Args,
};
use core::ffi::c_int;
use utils::{
	collections::path::PathBuf,
	errno,
	errno::{EResult, Errno},
	ptr::arc::Arc,
};

pub fn fchmodat(
	Args((dirfd, pathname, mode, flags)): Args<(c_int, SyscallString, file::Mode, c_int)>,
	fds_mutex: Arc<Mutex<FileDescriptorTable>>,
	rs: ResolutionSettings,
) -> EResult<usize> {
	let pathname = pathname
		.copy_from_user()?
		.map(PathBuf::try_from)
		.transpose()?;
	// Get file
	let fds = fds_mutex.lock();
	let Resolved::Found(file) = at::get_file(&fds, rs.clone(), dirfd, pathname.as_deref(), flags)?
	else {
		return Err(errno!(ENOENT));
	};
	// Check permission
	let stat = file.stat();
	if !rs.access_profile.can_set_file_permissions(&stat) {
		return Err(errno!(EPERM));
	}
	vfs::set_stat(
		file.node(),
		&StatSet {
			mode: Some(mode & 0o7777),
			..Default::default()
		},
	)?;
	Ok(0)
}
