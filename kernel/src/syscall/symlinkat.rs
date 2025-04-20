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

//! The `symlinkat` syscall allows to create a symbolic link.

use super::util::at;
use crate::{
	file::{
		fd::FileDescriptorTable,
		vfs,
		vfs::{ResolutionSettings, Resolved},
		FileType, Stat,
	},
	process::{mem_space::copy::SyscallString, Process},
	sync::mutex::Mutex,
	syscall::Args,
	time::clock::{current_time_ns, current_time_sec, Clock},
};
use core::ffi::c_int;
use utils::{
	collections::path::PathBuf,
	errno,
	errno::{EResult, Errno},
	limits::SYMLINK_MAX,
	ptr::arc::Arc,
};

pub fn symlinkat(
	Args((target, newdirfd, linkpath)): Args<(SyscallString, c_int, SyscallString)>,
	rs: ResolutionSettings,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	let target_slice = target.copy_from_user()?.ok_or_else(|| errno!(EFAULT))?;
	if target_slice.len() > SYMLINK_MAX {
		return Err(errno!(ENAMETOOLONG));
	}
	let target = PathBuf::try_from(target_slice)?;
	let linkpath = linkpath
		.copy_from_user()?
		.map(PathBuf::try_from)
		.transpose()?;
	let rs = ResolutionSettings {
		create: true,
		follow_link: true,
		..rs
	};
	// Create link
	let Resolved::Creatable {
		parent,
		name,
	} = at::get_file(&fds.lock(), rs.clone(), newdirfd, linkpath.as_deref(), 0)?
	else {
		return Err(errno!(EEXIST));
	};
	let ts = current_time_sec(Clock::Realtime);
	vfs::symlink(
		&parent,
		name,
		target.as_bytes(),
		&rs.access_profile,
		Stat {
			ctime: ts,
			mtime: ts,
			atime: ts,
			..Default::default()
		},
	)?;
	Ok(0)
}
