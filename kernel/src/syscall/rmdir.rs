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

//! The `rmdir` system call a link to the given directory from its filesystem.
//!
//! If no link remain to the directory, the function also removes it.

use crate::{
	file::{vfs, vfs::ResolutionSettings, FileType},
	process::{mem_space::copy::SyscallString, Process},
	syscall::Args,
};
use utils::{
	collections::path::PathBuf,
	errno,
	errno::{EResult, Errno},
};

pub fn rmdir(Args(pathname): Args<SyscallString>, rs: ResolutionSettings) -> EResult<usize> {
	let path = pathname.copy_from_user()?.ok_or(errno!(EFAULT))?;
	let path = PathBuf::try_from(path)?;
	vfs::unlink_from_path(&path, &rs)?;
	Ok(0)
}
