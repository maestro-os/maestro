//! The `openat` syscall allows to open a file.

use super::util;
use crate::errno::Errno;
use crate::file;
use crate::file::open_file;
use crate::file::File;
use crate::file::FileContent;
use crate::file::FileType;
use crate::file::Mode;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::Process;
use crate::syscall::openat::open_file::FDTarget;
use crate::util::ptr::SharedPtr;
use core::ffi::c_int;
use macros::syscall;

// TODO Implement all flags

/// Returns the file at the given path `path`.
/// TODO doc all args
/// If the file doesn't exist and the O_CREAT flag is set, the file is created,
/// then the function returns it. If the flag is not set, the function returns
/// an error with the appropriate errno. If the file is to be created, the
/// function uses `mode` to set its permissions.
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

/// The implementation of the `openat` syscall.
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

	{
		let guard = file.lock();
		let f = guard.get();

		// Checking file permissions
		let access = match flags & 0b11 {
			open_file::O_RDONLY => f.can_read(uid, gid),
			open_file::O_WRONLY => f.can_write(uid, gid),
			open_file::O_RDWR => f.can_read(uid, gid) && f.can_write(uid, gid),

			_ => true,
		};
		if !access {
			return Err(errno!(EACCES));
		}

		// If O_DIRECTORY is set and the file is not a directory, return an error
		if flags & open_file::O_DIRECTORY != 0 && f.get_type() != FileType::Directory {
			return Err(errno!(ENOTDIR));
		}
	}

	// Create and return the file descriptor
	let mutex = Process::get_current().unwrap();
	let guard = mutex.lock();
	let proc = guard.get_mut();
	let fd = proc.create_fd(flags & super::open::STATUS_FLAGS_MASK, FDTarget::File(file))?;
	Ok(fd.get_id() as _)
}
