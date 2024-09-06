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

//! The `open` system call allows a process to open a file and get a file
//! descriptor.

use super::{openat, Args};
use crate::{
	file,
	file::{
		fd::FD_CLOEXEC,
		path::{Path, PathBuf},
		perm::AccessProfile,
		vfs,
		vfs::{ResolutionSettings, Resolved},
		File, FileType, Stat,
	},
	process::{mem_space::copy::SyscallString, Process},
	syscall::{openat::do_openat, util::at::AT_FDCWD},
	time::{
		clock::{current_time, CLOCK_REALTIME},
		unit::TimestampScale,
	},
};
use core::ffi::c_int;
use utils::{
	errno,
	errno::{EResult, Errno},
	lock::Mutex,
	ptr::arc::Arc,
};

pub fn open(
	Args((pathname, flags, mode)): Args<(SyscallString, c_int, file::Mode)>,
) -> EResult<usize> {
	do_openat(AT_FDCWD, pathname, flags, mode)
}
