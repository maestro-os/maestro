//! The fchdir system call allows to change the current working directory of the current process.

use crate::errno::Errno;
use crate::errno;
use crate::file::FileType;
use crate::file::file_descriptor::FDTarget;
use crate::process::Process;
use crate::process::regs::Regs;

/// The implementation of the `fchdir` syscall.
pub fn fchdir(regs: &Regs) -> Result<i32, Errno> {
	let fd = regs.ebx as i32;

	let mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock();
	let proc = guard.get_mut();

	let uid = proc.get_euid();
	let gid = proc.get_egid();

	if fd < 0 {
		return Err(errno!(EBADF));
	}

	let fd = proc.get_fd(fd as _).ok_or_else(|| errno!(EBADF))?;
	if let FDTarget::File(dir_mutex) = fd.get_target_mut() {
		let new_cwd = {
			let mut dir_guard = dir_mutex.lock();
			let dir = dir_guard.get_mut();

			// Checking for errors
			if !dir.can_read(uid, gid) {
				return Err(errno!(EACCES));
			}
			if dir.get_file_type() != FileType::Directory {
				return Err(errno!(ENOTDIR));
			}

			dir.get_path()
		}?;

		let new_cwd = super::util::get_absolute_path(proc, new_cwd)?;
		proc.set_cwd(new_cwd)?;
		Ok(0)
	} else {
		Err(errno!(ENOTDIR))
	}
}
