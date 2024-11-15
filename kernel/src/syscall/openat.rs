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
		perm::AccessProfile,
		vfs,
		vfs::{ResolutionSettings, Resolved},
		File, FileType, Stat, O_CLOEXEC, O_CREAT, O_DIRECTORY, O_EXCL, O_NOCTTY, O_NOFOLLOW,
		O_RDONLY, O_RDWR, O_TRUNC, O_WRONLY,
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
	collections::path::{Path, PathBuf},
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
	path: Option<&Path>,
	flags: c_int,
	rs: ResolutionSettings,
	mode: file::Mode,
) -> EResult<Arc<vfs::Entry>> {
	let resolved = at::get_file(fds, rs.clone(), dirfd, path, flags)?;
	match resolved {
		Resolved::Found(file) => Ok(file),
		Resolved::Creatable {
			parent,
			name,
		} => {
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
	}
}

/// Perform the `openat` system call.
pub fn do_openat(
	dirfd: c_int,
	pathname: SyscallString,
	flags: c_int,
	mode: file::Mode,
) -> EResult<usize> {
	let (rs, pathname, fds_mutex, mode) = {
		let proc = Process::current();
		let follow_link = flags & O_NOFOLLOW == 0;
		let rs = ResolutionSettings {
			create: flags & O_CREAT != 0,
			..ResolutionSettings::for_process(&proc, follow_link)
		};
		let pathname = pathname
			.copy_from_user()?
			.map(PathBuf::try_from)
			.ok_or_else(|| errno!(EFAULT))??;
		let fds_mutex = proc.file_descriptors.clone().unwrap();
		let mode = mode & !proc.fs.lock().umask();
		(rs, pathname, fds_mutex, mode)
	};

	let mut fds = fds_mutex.lock();

	// Get file
	let file = get_file(&fds, dirfd, Some(&pathname), flags, rs.clone(), mode)?;
	// Check permissions
	let (read, write) = match flags & 0b11 {
		O_RDONLY => (true, false),
		O_WRONLY => (false, true),
		O_RDWR => (true, true),
		_ => return Err(errno!(EINVAL)),
	};
	let stat = file.stat()?;
	if read && !rs.access_profile.can_read_file(&stat) {
		return Err(errno!(EACCES));
	}
	if write && !rs.access_profile.can_write_file(&stat) {
		return Err(errno!(EACCES));
	}
	let file_type = stat.get_type();
	// If `O_DIRECTORY` is set and the file is not a directory, return an error
	if flags & O_DIRECTORY != 0 && file_type != Some(FileType::Directory) {
		return Err(errno!(ENOTDIR));
	}
	// Open file
	const FLAGS_MASK: i32 =
		!(O_CLOEXEC | O_CREAT | O_DIRECTORY | O_EXCL | O_NOCTTY | O_NOFOLLOW | O_TRUNC);
	let file = File::open_entry(file, flags & FLAGS_MASK)?;
	// Truncate if necessary
	if flags & O_TRUNC != 0 && file_type == Some(FileType::Regular) {
		file.truncate(0)?;
	}
	// Create FD
	let mut fd_flags = 0;
	if flags & O_CLOEXEC != 0 {
		fd_flags |= FD_CLOEXEC;
	}
	let (fd_id, _) = fds.create_fd(fd_flags, file)?;
	Ok(fd_id as _)
}

pub fn openat(
	Args((dirfd, pathname, flags, mode)): Args<(c_int, SyscallString, c_int, file::Mode)>,
) -> EResult<usize> {
	do_openat(dirfd, pathname, flags, mode)
}
