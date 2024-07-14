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

//! The `statfs` system call returns information about a mounted file system.

use crate::{
	file::{fs::Statfs, path::PathBuf, vfs, vfs::ResolutionSettings},
	process::{
		mem_space::copy::{SyscallPtr, SyscallString},
		Process,
	},
	syscall::Args,
};
use utils::{
	errno,
	errno::{EResult, Errno},
};

pub(super) fn do_statfs(path: SyscallString, buf: SyscallPtr<Statfs>) -> EResult<usize> {
	let (path, rs) = {
		let proc_mutex = Process::current();
		let proc = proc_mutex.lock();

		let path = path.copy_from_user()?.ok_or_else(|| errno!(EFAULT))?;
		let path = PathBuf::try_from(path)?;

		let rs = ResolutionSettings::for_process(&proc, false);
		(path, rs)
	};

	let stat = {
		let file_mutex = vfs::get_file_from_path(&path, &rs)?;
		let file = file_mutex.lock();

		// Unwrapping will not fail since the file is accessed from path
		let mountpoint_mutex = file.location.get_mountpoint().unwrap();
		let mountpoint = mountpoint_mutex.lock();

		let fs = mountpoint.get_filesystem();
		fs.get_stat()?
	};

	// Write structure to userspace
	buf.copy_to_user(stat)?;

	Ok(0)
}

pub fn statfs(Args((path, buf)): Args<(SyscallString, SyscallPtr<Statfs>)>) -> EResult<usize> {
	do_statfs(path, buf)
}
