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
		vfs,
		vfs::{ResolutionSettings, Resolved},
		FileType,
	},
	process::{mem_space::copy::UserString, Process},
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

pub fn linkat(
	Args((olddirfd, oldpath, newdirfd, newpath, flags)): Args<(
		c_int,
		UserString,
		c_int,
		UserString,
		c_int,
	)>,
	fds_mutex: Arc<Mutex<FileDescriptorTable>>,
	rs: ResolutionSettings,
) -> EResult<usize> {
	let oldpath = oldpath
		.copy_from_user()?
		.map(PathBuf::try_from)
		.ok_or_else(|| errno!(EFAULT))??;
	let newpath = newpath
		.copy_from_user()?
		.map(PathBuf::try_from)
		.ok_or_else(|| errno!(EFAULT))??;
	let fds = fds_mutex.lock();
	let rs = ResolutionSettings {
		follow_link: false,
		..rs
	};
	// Get old file
	let Resolved::Found(old) = at::get_file(&fds, rs.clone(), olddirfd, Some(&oldpath), flags)?
	else {
		return Err(errno!(ENOENT));
	};
	if old.get_type()? == FileType::Directory {
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
	} = at::get_file(&fds, rs.clone(), newdirfd, Some(&newpath), 0)?
	else {
		return Err(errno!(EEXIST));
	};
	let name = new_name.try_into()?;
	vfs::link(&new_parent, name, old.node().clone(), &rs.access_profile)?;
	Ok(0)
}
