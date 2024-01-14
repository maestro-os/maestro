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

use super::access::AT_FDCWD;
use super::util;
use crate::errno::Errno;
use crate::file::File;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::Process;
use crate::time::unit::TimeUnit;
use crate::time::unit::Timespec;
use crate::util::lock::Mutex;
use core::ffi::c_int;
use macros::syscall;

#[syscall]
pub fn utimensat(
	dirfd: c_int,
	pathname: SyscallString,
	times: SyscallPtr<[Timespec; 2]>,
	flags: c_int,
) -> Result<i32, Errno> {
	let proc_mutex = Process::current_assert();
	let proc = proc_mutex.lock();

	let mem_space = proc.get_mem_space().unwrap().clone();
	let mem_space_guard = mem_space.lock();

	let times_val = times.get(&mem_space_guard)?.ok_or(errno!(EFAULT))?;
	let atime = times_val[0];
	let mtime = times_val[1];

	let set = |file_mutex: &Mutex<File>| {
		let mut file = file_mutex.lock();
		// TODO clean
		file.atime = atime.to_nano() / 1000000000;
		file.mtime = mtime.to_nano() / 1000000000;
		// TODO sync only when required
		file.sync()
	};

	match pathname.get(&mem_space_guard)? {
		Some(pathname) => {
			let file_mutex = util::get_file_at(proc, dirfd, pathname, true, flags)?;
			set(&file_mutex)?;
		}
		None if dirfd != AT_FDCWD => {
			if dirfd < 0 {
				return Err(errno!(EBADF));
			}

			let fds = proc.file_descriptors.as_ref().unwrap().lock();
			let fd = fds.get_fd(dirfd as _).ok_or(errno!(EBADF))?;
			let open_file = fd.get_open_file().lock();
			set(open_file.get_file())?;
		}
		_ => return Err(errno!(EFAULT)),
	}
	Ok(0)
}
