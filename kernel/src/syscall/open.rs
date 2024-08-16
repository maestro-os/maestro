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

use super::Args;
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

/// Mask of status flags to be kept by an open file description.
pub const STATUS_FLAGS_MASK: i32 = !(file::O_CLOEXEC
	| file::O_CREAT
	| file::O_DIRECTORY
	| file::O_EXCL
	| file::O_NOCTTY
	| file::O_NOFOLLOW
	| file::O_TRUNC);

// TODO Implement all flags

/// Resolves the given `path` and returns the file.
///
/// The function creates the file if requested and required.
///
/// If the file is created, the function uses `mode` to set its permissions and the provided
/// access profile to set the user ID and group ID.
fn get_file(path: &Path, rs: &ResolutionSettings, mode: file::Mode) -> EResult<Arc<Mutex<File>>> {
	let resolved = vfs::resolve_path(path, rs)?;
	let file = match resolved {
		Resolved::Found(file) => file,
		Resolved::Creatable {
			parent,
			name,
		} => {
			let mut parent = parent.lock();
			let ts = current_time(CLOCK_REALTIME, TimestampScale::Second)?;
			vfs::create_file(
				&mut parent,
				name,
				&rs.access_profile,
				Stat {
					mode: FileType::Regular.to_mode() | mode,
					ctime: ts,
					mtime: ts,
					atime: ts,
					..Default::default()
				},
			)?
		}
	};
	// Get file type. There cannot be a race condition since the type of file cannot be
	// changed
	let file_type = file.lock().get_type()?;
	// Cannot open symbolic links themselves
	if file_type == FileType::Link {
		return Err(errno!(ELOOP));
	}
	Ok(file)
}

/// Checks the system call's flags and performs the action associated with some of them.
///
/// Arguments:
/// - `file` is the file
/// - `flags` is the set of flags provided by userspace
/// - `access_profile` is the access profile to check permissions
pub fn handle_flags(file: &mut File, flags: i32, access_profile: &AccessProfile) -> EResult<()> {
	let (read, write) = match flags & 0b11 {
		file::O_RDONLY => (true, false),
		file::O_WRONLY => (false, true),
		file::O_RDWR => (true, true),
		_ => return Err(errno!(EINVAL)),
	};
	let stat = file.get_stat()?;
	// Check access
	if read && !access_profile.can_read_file(&stat) {
		return Err(errno!(EACCES));
	}
	if write && !access_profile.can_write_file(&stat) {
		return Err(errno!(EACCES));
	}
	// If O_DIRECTORY is set and the file is not a directory, return an error
	if flags & file::O_DIRECTORY != 0 && stat.get_type() != Some(FileType::Directory) {
		return Err(errno!(ENOTDIR));
	}
	// Truncate the file if necessary
	if flags & file::O_TRUNC != 0 {
		file.truncate(0)?;
	}
	Ok(())
}

/// Performs the open system call.
pub fn open_(pathname: SyscallString, flags: i32, mode: file::Mode) -> EResult<usize> {
	let proc_mutex = Process::current();
	let (path, rs, mode, fds_mutex) = {
		let proc = proc_mutex.lock();

		let path = pathname.copy_from_user()?.ok_or(errno!(EFAULT))?;
		let path = PathBuf::try_from(path)?;

		let follow_links = flags & file::O_NOFOLLOW == 0;
		let create = flags & file::O_CREAT != 0;
		let mut rs = ResolutionSettings::for_process(&proc, follow_links);
		rs.create = create;

		let mode = mode & !proc.umask;

		let fds_mutex = proc.file_descriptors.clone().unwrap();

		(path, rs, mode, fds_mutex)
	};
	// Get file
	let file = get_file(&path, &rs, mode)?;
	handle_flags(&mut file.lock(), flags, &rs.access_profile)?;
	// Create FD
	let mut fd_flags = 0;
	if flags & file::O_CLOEXEC != 0 {
		fd_flags |= FD_CLOEXEC;
	}
	let (fd_id, _) = fds_mutex.lock().create_fd(fd_flags, file)?;
	Ok(fd_id as _)
}

pub fn open(
	Args((pathname, flags, mode)): Args<(SyscallString, c_int, file::Mode)>,
) -> EResult<usize> {
	open_(pathname, flags, mode)
}
