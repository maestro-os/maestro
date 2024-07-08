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
		path::{Path, PathBuf},
		vfs,
		vfs::{ResolutionSettings, Resolved},
		FileType, Stat,
	},
	limits,
	process::{mem_space::copy::SyscallString, Process},
	syscall::Args,
	time::{
		clock::{current_time, CLOCK_REALTIME},
		unit::TimestampScale,
	},
};
use core::ffi::c_int;
use utils::{
	errno,
	errno::{EResult, Errno},
	io::IO,
};

pub fn symlinkat(
	Args((target, newdirfd, linkpath)): Args<(SyscallString, c_int, SyscallString)>,
) -> EResult<usize> {
	let proc_mutex = Process::current_assert();
	let proc = proc_mutex.lock();

	let rs = ResolutionSettings::for_process(&proc, true);

	let mem_space = proc.get_mem_space().unwrap().clone();
	let mem_space_guard = mem_space.lock();

	let fds_mutex = proc.file_descriptors.clone().unwrap();
	let fds = fds_mutex.lock();

	let target_slice = target
		.copy_from_user(&mem_space_guard)?
		.ok_or_else(|| errno!(EFAULT))?;
	if target_slice.len() > limits::SYMLINK_MAX {
		return Err(errno!(ENAMETOOLONG));
	}
	let target = PathBuf::try_from(target_slice)?;

	let linkpath = linkpath
		.copy_from_user(&mem_space_guard)?
		.ok_or_else(|| errno!(EFAULT))?;
	let linkpath = PathBuf::try_from(linkpath)?;

	// Create link
	let resolved = at::get_file(&fds, rs.clone(), newdirfd, &linkpath, 0)?;
	match resolved {
		Resolved::Creatable {
			parent,
			name,
		} => {
			let mut parent = parent.lock();
			let ts = current_time(CLOCK_REALTIME, TimestampScale::Second)?;
			let file = vfs::create_file(
				&mut parent,
				name,
				&rs.access_profile,
				Stat {
					file_type: FileType::Link,
					mode: 0o777,
					ctime: ts,
					mtime: ts,
					atime: ts,
					..Default::default()
				},
			)?;
			file.lock().write(0, target.as_bytes())?;
		}
		Resolved::Found(_) => return Err(errno!(EEXIST)),
	}

	Ok(0)
}
