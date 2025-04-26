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

//! The `chmod` system call allows change the permissions on a file.

use crate::{
	file,
	file::{fd::FileDescriptorTable, fs::StatSet, vfs, vfs::ResolutionSettings},
	process::{mem_space::copy::UserString, Process},
	sync::mutex::Mutex,
	syscall::{fchmodat::fchmodat, util::at::AT_FDCWD, Args},
};
use core::ffi::c_int;
use utils::{
	collections::path::PathBuf,
	errno,
	errno::{EResult, Errno},
	ptr::arc::Arc,
};

pub fn chmod(
	Args((pathname, mode)): Args<(UserString, file::Mode)>,
	fds_mutex: Arc<Mutex<FileDescriptorTable>>,
	rs: ResolutionSettings,
) -> EResult<usize> {
	fchmodat(Args((AT_FDCWD, pathname, mode, 0)), fds_mutex, rs)
}
