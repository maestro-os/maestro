//! The fchdir system call allows to change the current working directory of the current process.

use crate::errno::Errno;
use crate::errno;
use crate::file::FileType;
use crate::file::file_descriptor::FDTarget;
use crate::process::Process;
use crate::process::Regs;

/// The implementation of the `fchdir` syscall.
pub fn fchdir(regs: &Regs) -> Result<i32, Errno> {
	let fd = regs.ebx as i32;

	let mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock();
	let proc = guard.get_mut();

	if fd < 0 {
		return Err(errno::EBADF);
	}

	if let Some(fd) = proc.get_fd(fd as _) {
		if let FDTarget::File(dir_mutex) = fd.get_target_mut() {
			let new_cwd = {
				let mut dir_guard = dir_mutex.lock();
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
			return Err(errno::ENOTDIR);
		}
	} else {
		Err(errno::EBADF)
	}
}
