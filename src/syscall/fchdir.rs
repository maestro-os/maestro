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
		let open_file_mutex = proc
			.get_fd(fd as _)
			.ok_or_else(|| errno!(EBADF))?
			.get_open_file();

		(uid, gid, open_file_mutex)
	};

	let open_file_guard = open_file_mutex.lock();
	let open_file = open_file_guard.get_mut();

	if let FDTarget::File(dir_mutex) = open_file.get_target_mut() {
		let new_cwd = {
			let dir_guard = dir_mutex.lock();
			let dir = dir_guard.get_mut();

			// Checking for errors
			if !dir.can_read(uid, gid) {
				return Err(errno!(EACCES));
			}
			if dir.get_type() != FileType::Directory {
				return Err(errno!(ENOTDIR));
			}

			dir.get_path()
		}?;

		{
			let mutex = Process::get_current().unwrap();
			let guard = mutex.lock();
			let proc = guard.get_mut();

			let new_cwd = super::util::get_absolute_path(proc, new_cwd)?;
			proc.set_cwd(new_cwd)?;
		}

		Ok(0)
	} else {
		Err(errno!(ENOTDIR))
	}
}
