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

//! The `utimensat` system call allows to change the timestamps of a file.

use super::util::at;
use crate::{
	file::{
		fd::FileDescriptorTable,
		fs::StatSet,
		path::{Path, PathBuf},
		vfs::{ResolutionSettings, Resolved},
	},
	process::{
		mem_space::copy::{SyscallPtr, SyscallString},
		Process,
	},
	syscall::Args,
	time::unit::{TimeUnit, Timespec},
	tty::vga::DEFAULT_COLOR,
};
use core::ffi::c_int;
use utils::{
	errno,
	errno::{EResult, Errno},
	lock::Mutex,
	ptr::arc::Arc,
};

pub fn utimensat(
	Args((dirfd, pathname, times, flags)): Args<(
		c_int,
		SyscallString,
		SyscallPtr<[Timespec; 2]>,
		c_int,
	)>,
	rs: ResolutionSettings,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	let pathname = pathname.copy_from_user()?.ok_or_else(|| errno!(EFAULT))?;
	let pathname = PathBuf::try_from(pathname)?;
	let times_val = times.copy_from_user()?.ok_or_else(|| errno!(EFAULT))?;
	let atime = times_val[0];
	let mtime = times_val[1];
	// Get file
	let Resolved::Found(file) = at::get_file(&fds.lock(), rs, dirfd, &pathname, flags)? else {
		return Err(errno!(ENOENT));
	};
	// Update timestamps
	file.ops.set_stat(
		&file.location,
		StatSet {
			atime: Some(atime.to_nano() / 1000000000),
			mtime: Some(mtime.to_nano() / 1000000000),
			..Default::default()
		},
	)?;
	Ok(0)
}
