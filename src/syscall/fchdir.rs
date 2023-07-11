//! The fchdir system call allows to change the current working directory of the
//! current process.

use crate::errno;
use crate::errno::Errno;
use crate::file::FileType;
use crate::process::Process;
use core::ffi::c_int;
use macros::syscall;

#[syscall]
pub fn fchdir(fd: c_int) -> Result<i32, Errno> {
	if fd < 0 {
		return Err(errno!(EBADF));
	}

	let (uid, gid, open_file_mutex) = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let uid = proc.euid;
		let gid = proc.egid;

		let fds_mutex = proc.get_fds().unwrap();
		let fds = fds_mutex.lock();

		let open_file_mutex = fds
			.get_fd(fd as _)
			.ok_or_else(|| errno!(EBADF))?
			.get_open_file()?;

		(uid, gid, open_file_mutex)
	};

	let open_file = open_file_mutex.lock();

	let new_cwd = {
		let file_mutex = open_file.get_file()?;
		let file = file_mutex.lock();

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
		let proc_mutex = Process::current_assert();
		let mut proc = proc_mutex.lock();

		let new_cwd = super::util::get_absolute_path(&proc, new_cwd)?;
		proc.set_cwd(new_cwd)?;
	}

	Ok(0)
}
