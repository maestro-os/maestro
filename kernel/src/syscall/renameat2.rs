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

//! The `renameat2` allows to rename a file.

use super::util::at;
use crate::{
	file::{
		FileType,
		fd::FileDescriptorTable,
		vfs,
		vfs::{ResolutionSettings, Resolved},
	},
	memory::user::UserString,
	process::Process,
	sync::mutex::Mutex,
	syscall::Args,
};
use core::{ffi::c_int, ptr};
use utils::{
	collections::path::PathBuf,
	errno,
	errno::{EResult, Errno},
	ptr::arc::Arc,
};

/// Flag: Don't replace new path if it exists. Return an error instead.
const RENAME_NOREPLACE: c_int = 1;
/// Flag: Exchanges old and new paths atomically.
const RENAME_EXCHANGE: c_int = 2;

// TODO implement flags

pub(super) fn do_renameat2(
	olddirfd: c_int,
	oldpath: UserString,
	newdirfd: c_int,
	newpath: UserString,
	_flags: c_int,
	fds: Arc<Mutex<FileDescriptorTable>>,
	rs: ResolutionSettings,
) -> EResult<usize> {
	let rs = ResolutionSettings {
		follow_link: false,
		..rs
	};
	// Get old file
	let oldpath = oldpath
		.copy_from_user()?
		.map(PathBuf::try_from)
		.ok_or_else(|| errno!(EFAULT))??;
	let Resolved::Found(old) = at::get_file(&fds.lock(), rs.clone(), olddirfd, Some(&oldpath), 0)?
	else {
		return Err(errno!(ENOENT));
	};
	// Get new file
	let newpath = newpath
		.copy_from_user()?
		.map(PathBuf::try_from)
		.ok_or_else(|| errno!(EFAULT))??;
	let rs = ResolutionSettings {
		create: true,
		..rs
	};
	let res = at::get_file(&fds.lock(), rs.clone(), newdirfd, Some(&newpath), 0)?;
	match res {
		Resolved::Found(new) => {
			// cannot move the root of the vfs
			let new_parent = new.parent.clone().ok_or_else(|| errno!(EBUSY))?;
			vfs::rename(old, new_parent, &new.name, &rs.access_profile)?;
		}
		Resolved::Creatable {
			parent: new_parent,
			name: new_name,
		} => vfs::rename(old, new_parent, new_name, &rs.access_profile)?,
	}
	Ok(0)
}

pub fn renameat2(
	Args((olddirfd, oldpath, newdirfd, newpath, flags)): Args<(
		c_int,
		UserString,
		c_int,
		UserString,
		c_int,
	)>,
	fds: Arc<Mutex<FileDescriptorTable>>,
	rs: ResolutionSettings,
) -> EResult<usize> {
	do_renameat2(olddirfd, oldpath, newdirfd, newpath, flags, fds, rs)
}
