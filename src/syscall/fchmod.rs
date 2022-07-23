//! The `fchmod` system call allows change the permissions on a file.

use crate::file::open_file::FDTarget;
use crate::errno::Errno;
use crate::file;
use crate::process::Process;
use crate::process::regs::Regs;

/// The implementation of the `fchmod` syscall.
pub fn fchmod(regs: &Regs) -> Result<i32, Errno> {
	let fd = regs.ebx as i32;
	let mode = regs.ecx as file::Mode;

	if fd < 0 {
		return Err(errno!(EBADF));
	}

	let (file_mutex, uid) = {
		let mutex = Process::get_current().unwrap();
		let guard = mutex.lock();
		let proc = guard.get_mut();

		let fd = proc.get_fd(fd as _).ok_or_else(|| errno!(EBADF))?;
		let open_file = fd.get_open_file();
		let open_file_guard = open_file.lock();

		let file_mutex = match open_file_guard.get().get_target() {
			FDTarget::File(file) => file.clone(),

			_ => return Err(errno!(EPERM)),
		};

		(file_mutex, proc.get_euid())
	};
	let file_guard = file_mutex.lock();
	let file = file_guard.get_mut();

	// Checking permissions
	if uid != file::ROOT_UID && uid != file.get_uid() {
		return Err(errno!(EPERM));
	}

	file.set_permissions(mode);
	file.sync()?;

	Ok(0)
}
