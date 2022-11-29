//! The fchdir system call allows to change the current working directory of the
//! current process.

use crate::errno;
use crate::errno::Errno;
use crate::file::FileType;
use crate::process::Process;
use core::ffi::c_int;
use macros::syscall;

/// The implementation of the `fchdir` syscall.
#[syscall]
pub fn fchdir(fd: c_int) -> Result<i32, Errno> {
	if fd < 0 {
		return Err(errno!(EBADF));
	}

	let (uid, gid, open_file_mutex) = {
		let mutex = Process::get_current().unwrap();
		let guard = mutex.lock();
		let proc = guard.get_mut();

		let uid = proc.get_euid();
		let gid = proc.get_egid();

		let fds_mutex = proc.get_fds().unwrap();
		let fds_guard = fds_mutex.lock();
		let fds = fds_guard.get();

		let open_file_mutex = fds
			.get_fd(fd as _)
			.ok_or_else(|| errno!(EBADF))?
			.get_open_file();

		(uid, gid, open_file_mutex)
	};

	let open_file_guard = open_file_mutex.lock();
	let open_file = open_file_guard.get_mut();

	let new_cwd = {
		let file_mutex = open_file.get_file()?;
		let file_guard = file_mutex.lock();
		let file = file_guard.get_mut();

		// Checking for errors
		if !file.can_read(uid, gid) {
			return Err(errno!(EACCES));
		}
		if file.get_type() != FileType::Directory {
			return Err(errno!(ENOTDIR));
		}

		file.get_path()
	}?;

	{
		let mutex = Process::get_current().unwrap();
		let guard = mutex.lock();
		let proc = guard.get_mut();

		let new_cwd = super::util::get_absolute_path(proc, new_cwd)?;
		proc.set_cwd(new_cwd)?;
	}

	Ok(0)
}
