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
		path::PathBuf,
		vfs::{ResolutionSettings, Resolved},
	},
	process::{mem_space::copy::SyscallString, Process},
	syscall::Args,
};
use core::ffi::c_int;
use utils::{
	errno,
	errno::{EResult, Errno},
};

pub fn fchmodat(
	Args((dirfd, pathname, mode, flags)): Args<(c_int, SyscallString, file::Mode, c_int)>,
) -> EResult<usize> {
	let (fds_mutex, path, rs) = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let rs = ResolutionSettings::for_process(&proc, true);

		let fds_mutex = proc.file_descriptors.clone().unwrap();

		let pathname = pathname.copy_from_user()?.ok_or_else(|| errno!(EFAULT))?;
		let path = PathBuf::try_from(pathname)?;

		(fds_mutex, path, rs)
	};

	let fds = fds_mutex.lock();

	let Resolved::Found(file_mutex) = at::get_file(&fds, rs.clone(), dirfd, &path, flags)? else {
		return Err(errno!(ENOENT));
	};
	let mut file = file_mutex.lock();

	// Check permission
	if !rs.access_profile.can_set_file_permissions(&file) {
		return Err(errno!(EPERM));
	}

	file.stat.set_permissions(mode as _);
	// TODO lazy sync
	file.sync()?;

	Ok(0)
}
