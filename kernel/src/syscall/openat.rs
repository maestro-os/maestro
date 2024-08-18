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

use crate::{
	file,
	file::{
		fd::{FileDescriptorTable, FD_CLOEXEC},
		path::{Path, PathBuf},
		vfs,
		vfs::{ResolutionSettings, Resolved},
		File, FileType, Stat,
	},
	process::{mem_space::copy::SyscallString, Process},
	syscall::{util::at, Args},
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

// TODO Implement all flags

// TODO rewrite doc
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
	fds: &FileDescriptorTable,
	dirfd: c_int,
	path: &Path,
	flags: c_int,
	rs: ResolutionSettings,
	mode: file::Mode,
) -> EResult<Arc<vfs::Entry>> {
	let create = flags & file::O_CREAT != 0;
	let resolved = at::get_file(fds, rs.clone(), dirfd, path, flags)?;
	match resolved {
		Resolved::Found(file) => Ok(file),
		Resolved::Creatable {
			parent,
			name,
		} if create => {
			let ts = current_time(CLOCK_REALTIME, TimestampScale::Second)?;
			vfs::create_file(
				parent,
				name,
				&rs.access_profile,
				Stat {
					mode: FileType::Regular.to_mode() | mode,
					ctime: ts,
					mtime: ts,
					atime: ts,
					..Default::default()
				},
			)
		}
		_ => Err(errno!(ENOENT)),
	}
}

pub fn openat(
	Args((dirfd, pathname, flags, mode)): Args<(c_int, SyscallString, c_int, file::Mode)>,
) -> EResult<usize> {
	let (rs, path, fds_mutex, mode) = {
		let proc_mutex = Process::current();
		let proc = proc_mutex.lock();

		let follow_link = flags & file::O_NOFOLLOW == 0;
		let rs = ResolutionSettings::for_process(&proc, follow_link);

		let pathname = pathname.copy_from_user()?.ok_or_else(|| errno!(EFAULT))?;
		let path = PathBuf::try_from(pathname)?;

		let fds_mutex = proc.file_descriptors.clone().unwrap();

		let mode = mode & !proc.umask;

		(rs, path, fds_mutex, mode)
	};

	let mut fds = fds_mutex.lock();

	// Get file
	let file = get_file(&fds, dirfd, &path, flags, rs.clone(), mode)?;
	super::open::check_perms(&file, flags, &rs.access_profile)?;
	let file = File::open(file, flags)?;
	// Truncate the file if necessary
	if flags & file::O_TRUNC != 0 {
		file.lock().truncate(0)?;
	}
	// Create FD
	let mut fd_flags = 0;
	if flags & file::O_CLOEXEC != 0 {
		fd_flags |= FD_CLOEXEC;
	}
	let (fd_id, _) = fds.create_fd(fd_flags, file)?;
	Ok(fd_id as _)
}
