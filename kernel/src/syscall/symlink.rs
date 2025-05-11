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

//! The `symlink` syscall allows to create a symbolic link.

use crate::{
	file::{FileType, Stat, fd::FileDescriptorTable, vfs, vfs::ResolutionSettings},
	memory::user::UserString,
	sync::mutex::Mutex,
	syscall::{Args, symlinkat::symlinkat, util::at::AT_FDCWD},
	time::clock::current_time_ns,
};
use utils::{
	collections::path::{Path, PathBuf},
	errno,
	errno::{EResult, Errno},
	limits::SYMLINK_MAX,
	ptr::arc::Arc,
};

pub fn symlink(
	Args((target, linkpath)): Args<(UserString, UserString)>,
	rs: ResolutionSettings,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	symlinkat(Args((target, AT_FDCWD, linkpath)), rs, fds)
}
