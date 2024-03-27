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

//! The open system call allows a process to open a file and get a file
//! descriptor.

use crate::{
	file::{
		fd::FD_CLOEXEC,
		open_file,
		open_file::OpenFile,
		path::{Path, PathBuf},
		perm::AccessProfile,
		vfs,
		vfs::{ResolutionSettings, Resolved},
		File, FileType, Mode,
	},
	process::{mem_space::ptr::SyscallString, Process},
};
use core::ffi::c_int;
use macros::syscall;
use utils::{
	errno,
	errno::{EResult, Errno},
	lock::Mutex,
	ptr::arc::Arc,
};

/// Mask of status flags to be kept by an open file description.
pub const STATUS_FLAGS_MASK: i32 = !(open_file::O_CLOEXEC
	| open_file::O_CREAT
	| open_file::O_DIRECTORY
	| open_file::O_EXCL
	| open_file::O_NOCTTY
	| open_file::O_NOFOLLOW
	| open_file::O_TRUNC);

// TODO Implement all flags

/// Resolves the given `path` and returns the file.
///
/// If enabled, the file is create.
///
/// If the file is created, the function uses `mode` to set its permissions and the provided
/// access profile to set the user ID and group ID.
fn get_file(path: &Path, rs: &ResolutionSettings, mode: Mode) -> EResult<Arc<Mutex<File>>> {
	let resolved = vfs::resolve_path(path, rs)?;
	let file = match resolved {
		Resolved::Found(file) => file,
		Resolved::Creatable {
			parent,
			name,
		} => {
			let mut parent = parent.lock();
			let name = name.try_into()?;
			vfs::create_file(
				&mut parent,
				name,
				&rs.access_profile,
				mode,
				FileContent::Regular,
			)?
		}
	};
	// Get file type. There cannot be a race condition since the type of a file cannot be
	// changed
	let file_type = file.lock().get_type();
	// Cannot open symbolic links themselves
	if file_type == FileType::Link {
		return Err(errno!(ELOOP));
	}
	Ok(file)
}

/// The function checks the system call's flags and performs the action associated with some of
/// them.
///
/// Arguments:
/// - `file` is the file
/// - `flags` is the set of flags provided by userspace
/// - `access_profile` is the access profile to check permissions
pub fn handle_flags(file: &mut File, flags: i32, access_profile: &AccessProfile) -> EResult<()> {
	let (read, write) = match flags & 0b11 {
		open_file::O_RDONLY => (true, false),
		open_file::O_WRONLY => (false, true),
		open_file::O_RDWR => (true, true),
		_ => return Err(errno!(EINVAL)),
	};
	if read && !access_profile.can_read_file(file) {
		return Err(errno!(EACCES));
	}
	if write && !access_profile.can_write_file(file) {
		return Err(errno!(EACCES));
	}

	// If O_DIRECTORY is set and the file is not a directory, return an error
	if flags & open_file::O_DIRECTORY != 0 && file.get_type() != FileType::Directory {
		return Err(errno!(ENOTDIR));
	}
	// Truncate the file if necessary
	if flags & open_file::O_TRUNC != 0 {
		file.set_size(0);
	}

	Ok(())
}

/// Performs the open system call.
pub fn open_(pathname: SyscallString, flags: i32, mode: Mode) -> EResult<i32> {
	let proc_mutex = Process::current_assert();
	let (path, rs, mode, fds_mutex) = {
		let proc = proc_mutex.lock();

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();
		let path = pathname.get(&mem_space_guard)?.ok_or(errno!(EFAULT))?;
		let path = PathBuf::try_from(path)?;

		let follow_links = flags & open_file::O_NOFOLLOW == 0;
		let create = flags & open_file::O_CREAT != 0;
		let mut rs = ResolutionSettings::for_process(&proc, follow_links);
		rs.create = create;

		let mode = mode & !proc.umask;

		let fds_mutex = proc.file_descriptors.clone().unwrap();

		(path, rs, mode, fds_mutex)
	};

	// Get file
	let file_mutex = get_file(&path, &rs, mode)?;
	{
		let mut file = file_mutex.lock();
		handle_flags(&mut file, flags, &rs.access_profile)?;
	}

	// Create open file description
	let open_file = OpenFile::new(file_mutex.clone(), flags)?;

	// Create FD
	let mut fd_flags = 0;
	if flags & open_file::O_CLOEXEC != 0 {
		fd_flags |= FD_CLOEXEC;
	}
	let mut fds = fds_mutex.lock();
	let fd = fds.create_fd(fd_flags, open_file)?;
	let fd_id = fd.get_id();

	// TODO remove?
	// Flush file
	let file = file_mutex.lock();
	if let Err(e) = file.sync() {
		fds.close_fd(fd_id)?;
		return Err(e);
	}

	Ok(fd_id as _)
}

#[syscall]
pub fn open(pathname: SyscallString, flags: c_int, mode: Mode) -> Result<i32, Errno> {
	open_(pathname, flags, mode)
}
