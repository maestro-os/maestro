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

//! The `linkat` system call allows to create a hard link.

use super::util::at;
use crate::{
	file::{
		fd::FileDescriptorTable,
		path::PathBuf,
		vfs,
		vfs::{ResolutionSettings, Resolved},
		FileType,
	},
	process::{mem_space::copy::SyscallString, Process},
	syscall::Args,
};
use core::ffi::c_int;
use utils::{
	errno,
	errno::{EResult, Errno},
	lock::Mutex,
	ptr::arc::Arc,
};

pub fn linkat(
	Args((olddirfd, oldpath, newdirfd, newpath, flags)): Args<(
		c_int,
		SyscallString,
		c_int,
		SyscallString,
		c_int,
	)>,
	fds_mutex: Arc<Mutex<FileDescriptorTable>>,
	rs: ResolutionSettings,
) -> EResult<usize> {
	let oldpath = oldpath.copy_from_user()?.ok_or_else(|| errno!(EFAULT))?;
	let oldpath = PathBuf::try_from(oldpath)?;
	let newpath = newpath.copy_from_user()?.ok_or_else(|| errno!(EFAULT))?;
	let newpath = PathBuf::try_from(newpath)?;
	let fds = fds_mutex.lock();
	let rs = ResolutionSettings {
		follow_link: false,
		..rs
	};
	// Get old file
	let Resolved::Found(old_mutex) = at::get_file(&fds, rs.clone(), olddirfd, &oldpath, flags)?
	else {
		return Err(errno!(ENOENT));
	};
	let mut old = old_mutex.lock();
	if matches!(old.stat.file_type, FileType::Directory) {
		return Err(errno!(EPERM));
	}
	// Create new file
	let rs = ResolutionSettings {
		create: true,
		..rs
	};
	let Resolved::Creatable {
		parent: new_parent,
		name: new_name,
	} = at::get_file(&fds, rs.clone(), newdirfd, &newpath, 0)?
	else {
		return Err(errno!(EEXIST));
	};
	let new_parent = new_parent.lock();
	vfs::create_link(&new_parent, new_name, &mut old, &rs.access_profile)?;
	Ok(0)
}
