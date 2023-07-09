//! The `openat` syscall allows to open a file.

use super::util;
use crate::errno::Errno;
use crate::file;
use crate::file::fd::FD_CLOEXEC;
use crate::file::open_file;
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

	let mem_space = proc.get_mem_space().unwrap();
	let mem_space_guard = mem_space.lock();

	let pathname = pathname
		.get(&mem_space_guard)?
		.ok_or_else(|| errno!(EFAULT))?;

	if flags & open_file::O_CREAT != 0 {
		util::create_file_at(
			proc,
			follow_links,
			dirfd,
			pathname,
			mode,
			FileContent::Regular,
		)
	} else {
		util::get_file_at(proc, true, dirfd, pathname, 0)
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
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		(proc.euid, proc.egid)
	};

	let (loc, read, write, cloexec) = {
		let mut f = file.lock();

		let loc = f.get_location().clone();
		let (read, write, cloexec) = super::open::handle_flags(&mut f, flags, uid, gid)?;

		(loc, read, write, cloexec)
	};

	open_file::OpenFile::new(loc.clone(), flags)?;

	let proc_mutex = Process::current_assert();
	let proc = proc_mutex.lock();

	let fds_mutex = proc.get_fds().unwrap();
	let mut fds = fds_mutex.lock();

	let mut fd_flags = 0;
	if cloexec {
		fd_flags |= FD_CLOEXEC;
	}

	let fd = fds.create_fd(loc, fd_flags, read, write)?;
	Ok(fd.get_id() as _)
}
