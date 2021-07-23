//! The fchdir system call allows to change the current working directory of the current process.

use crate::errno::Errno;
use crate::errno;
use crate::file::FileType;
use crate::process::Process;
use crate::util;

/// The implementation of the `fchdir` syscall.
pub fn fchdir(regs: &util::Regs) -> Result<i32, Errno> {
	let fd = regs.ebx as i32;

	let mut mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock(false);
	let proc = guard.get_mut();

	if fd < 0 {
		return Err(errno::EBADF);
	}

	if let Some(fd) = proc.get_fd(fd as _) {
		let new_cwd = {
			let dir_mutex = fd.get_file_mut();
			let mut dir_guard = dir_mutex.lock(true);
			let dir = dir_guard.get_mut();

			// TODO Check permission
			if dir.get_file_type() != FileType::Directory {
				return Err(errno::ENOTDIR);
			}

			dir.get_path()
		}?;

		proc.set_cwd(new_cwd);
		Ok(0)
	} else {
		Err(errno::EBADF)
	}
}
