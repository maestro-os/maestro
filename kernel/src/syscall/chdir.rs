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

//! The chdir system call allows to change the current working directory of the
//! current process.

use crate::{
	file::{vfs, vfs::ResolutionSettings, FileType},
	process::{mem_space::copy::SyscallString, Process},
	syscall::Args,
};
use utils::{
	collections::path::PathBuf,
	errno,
	errno::{EResult, Errno},
	lock::IntMutex,
	ptr::arc::Arc,
};

pub fn chdir(
	Args(path): Args<SyscallString>,
	proc: Arc<Process>,
	rs: ResolutionSettings,
) -> EResult<usize> {
	let path = path.copy_from_user()?.ok_or_else(|| errno!(EFAULT))?;
	let path = PathBuf::try_from(path)?;
	// Get directory
	let dir = vfs::get_file_from_path(&path, &rs)?;
	// Validation
	let stat = dir.stat()?;
	if stat.get_type() != Some(FileType::Directory) {
		return Err(errno!(ENOTDIR));
	}
	if !rs.access_profile.can_list_directory(&stat) {
		return Err(errno!(EACCES));
	}
	// Set new cwd
	proc.fs.lock().cwd = dir;
	Ok(0)
}
