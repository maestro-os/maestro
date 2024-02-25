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

//! The `openat` syscall allows to open a file.

use super::util;
use crate::errno::Errno;
use crate::file;
use crate::file::fd::FD_CLOEXEC;
use crate::file::open_file;
use crate::file::open_file::OpenFile;
use crate::file::File;
use crate::file::FileContent;
use crate::file::Mode;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::Process;
use crate::util::lock::Mutex;
use crate::util::ptr::arc::Arc;
use core::ffi::c_int;
use macros::syscall;

// TODO Implement all flags
// TODO clean up: multiple locks to process

/// Returns the file at the given path.
///
/// Arguments:
/// - `dirfd` a file descriptor to the directory from which the file will be searched.
/// - `pathname` the path relative to the directory.
/// - `flags` is a set of open file flags.
/// - `mode` is the set of permissions to use if the file needs to be created.
///
/// If the file doesn't exist and the `O_CREAT` flag is set, the file is created,
/// then the function returns it.
///
/// If the flag is not set, the function returns an error with the appropriate errno.
///
/// If the file is to be created, the function uses `mode` to set its permissions.
fn get_file(
	dirfd: i32,
	pathname: SyscallString,
	flags: i32,
	mode: Mode,
) -> Result<Arc<Mutex<File>>, Errno> {
	// Tells whether to follow symbolic links on the last component of the path.
	let follow_links = flags & open_file::O_NOFOLLOW == 0;

	let proc_mutex = Process::current_assert();
	let proc = proc_mutex.lock();

	let mem_space = proc.get_mem_space().unwrap().clone();
	let mem_space_guard = mem_space.lock();

	let pathname = pathname
		.get(&mem_space_guard)?
		.ok_or_else(|| errno!(EFAULT))?;

	if flags & open_file::O_CREAT != 0 {
		util::create_file_at(
			proc,
			dirfd,
			pathname,
			mode,
			FileContent::Regular,
			follow_links,
			0,
		)
	} else {
		util::get_file_at(proc, dirfd, pathname, follow_links, 0)
	}
}

#[syscall]
pub fn openat(
	dirfd: c_int,
	pathname: SyscallString,
	flags: c_int,
	mode: file::Mode,
) -> Result<i32, Errno> {
	let proc_mutex = Process::current_assert();
	let ap = proc_mutex.lock().access_profile;

	// Get the file
	let file_mutex = get_file(dirfd, pathname, flags, mode)?;
	let mut file = file_mutex.lock();

	// Handle flags
	super::open::handle_flags(&mut file, flags, &ap)?;
	drop(file);

	let open_file = OpenFile::new(file_mutex, flags)?;

	let mut fd_flags = 0;
	if flags & open_file::O_CLOEXEC != 0 {
		fd_flags |= FD_CLOEXEC;
	}
	let proc = proc_mutex.lock();
	let fds_mutex = proc.get_fds().unwrap();
	let mut fds = fds_mutex.lock();
	let fd = fds.create_fd(fd_flags, open_file)?;

	Ok(fd.get_id() as _)
}
