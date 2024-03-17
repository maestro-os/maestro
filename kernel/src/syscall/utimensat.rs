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
		path::Path,
		vfs::{ResolutionSettings, Resolved},
	},
	process::{
		mem_space::ptr::{SyscallPtr, SyscallString},
		Process,
	},
	time::unit::{TimeUnit, Timespec},
};
use core::ffi::c_int;
use macros::syscall;
use utils::{errno, errno::Errno};

#[syscall]
pub fn utimensat(
	dirfd: c_int,
	pathname: SyscallString,
	times: SyscallPtr<[Timespec; 2]>,
	flags: c_int,
) -> Result<i32, Errno> {
	let proc_mutex = Process::current_assert();
	let proc = proc_mutex.lock();

	let rs = ResolutionSettings::for_process(&proc, true);

	let mem_space = proc.get_mem_space().unwrap().clone();
	let mem_space_guard = mem_space.lock();

	let fds = proc.file_descriptors.as_ref().unwrap().lock();

	let pathname = pathname
		.get(&mem_space_guard)?
		.ok_or_else(|| errno!(EFAULT))?;
	let pathname = Path::new(pathname)?;

	let times_val = times.get(&mem_space_guard)?.ok_or_else(|| errno!(EFAULT))?;
	let atime = times_val[0];
	let mtime = times_val[1];

	let Resolved::Found(file_mutex) = at::get_file(&fds, rs, dirfd, pathname, flags)? else {
		return Err(errno!(ENOENT));
	};
	let mut file = file_mutex.lock();

	// TODO clean
	file.atime = atime.to_nano() / 1000000000;
	file.mtime = mtime.to_nano() / 1000000000;
	// TODO sync only when required
	file.sync()?;

	Ok(0)
}
