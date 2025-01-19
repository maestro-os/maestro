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

//! The `link` system call allows to create a hard link.

use super::Args;
use crate::{
	file::{fd::FileDescriptorTable, vfs::ResolutionSettings},
	process::{mem_space::copy::SyscallString, Process},
	sync::mutex::Mutex,
	syscall::{linkat::linkat, util::at::AT_FDCWD},
};
use utils::{
	collections::path::PathBuf,
	errno,
	errno::{EResult, Errno},
	ptr::arc::Arc,
};

pub fn link(
	Args((oldpath, newpath)): Args<(SyscallString, SyscallString)>,
	fds_mutex: Arc<Mutex<FileDescriptorTable>>,
	rs: ResolutionSettings,
) -> EResult<usize> {
	linkat(
		Args((AT_FDCWD, oldpath, AT_FDCWD, newpath, 0)),
		fds_mutex,
		rs,
	)
}
