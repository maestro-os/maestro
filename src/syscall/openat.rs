//! The `openat` syscall allows to open a file.

use core::ffi::c_int;
use crate::errno::Errno;
use crate::file::File;
use crate::file::FileContent;
use crate::file::Mode;
use crate::file::fd::FD_CLOEXEC;
use crate::file::open_file;
use crate::file;
use crate::process::Process;
use crate::process::mem_space::ptr::SyscallString;
use crate::util::ptr::SharedPtr;
use macros::syscall;
use super::util;

// TODO Implement all flags

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
/// If the flag is not set, the function returns an error with the appropriate errno.
/// If the file is to be created, the function uses `mode` to set its permissions.
fn get_file(
	dirfd: i32,
	pathname: SyscallString,
	flags: i32,
	mode: Mode,
) -> Result<SharedPtr<File>, Errno> {
	// Tells whether to follow symbolic links on the last component of the path.
	let follow_links = flags & open_file::O_NOFOLLOW == 0;

	let proc_mutex = Process::get_current().unwrap();
	let proc_guard = proc_mutex.lock();
	let proc = proc_guard.get_mut();

	let mem_space = proc.get_mem_space().unwrap();
	let mem_space_guard = mem_space.lock();

	let pathname = pathname
		.get(&mem_space_guard)?
		.ok_or_else(|| errno!(EFAULT))?;

	if flags & open_file::O_CREAT != 0 {
		util::create_file_at(
			proc_guard,
			follow_links,
			dirfd,
			pathname,
			mode,
			FileContent::Regular,
		)
	} else {
		util::get_file_at(proc_guard, true, dirfd, pathname, 0)
	}
}

#[syscall]
pub fn openat(
	dirfd: c_int,
	pathname: SyscallString,
	flags: c_int,
	mode: file::Mode,
) -> Result<i32, Errno> {
	// Getting the file
	let file = get_file(dirfd, pathname, flags, mode)?;

	let (uid, gid) = {
		let mutex = Process::get_current().unwrap();
		let guard = mutex.lock();
		let proc = guard.get_mut();

		(proc.get_euid(), proc.get_egid())
	};

	let (loc, read, write, cloexec) = {
		let guard = file.lock();
		let f = guard.get_mut();

		let loc = f.get_location().clone();
		let (read, write, cloexec) = super::open::handle_flags(f, flags, uid, gid)?;

		(loc, read, write, cloexec)
	};

	open_file::OpenFile::new(loc.clone(), flags)?;

	// Create and return the file descriptor
	let mutex = Process::get_current().unwrap();
	let guard = mutex.lock();
	let proc = guard.get_mut();

	let fds_mutex = proc.get_fds().unwrap();
	let fds_guard = fds_mutex.lock();
	let fds = fds_guard.get_mut();

	let mut fd_flags = 0;
	if cloexec {
		fd_flags |= FD_CLOEXEC;
	}

	let fd = fds.create_fd(loc, fd_flags, read, write)?;
	Ok(fd.get_id() as _)
}
