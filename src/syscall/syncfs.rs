//! The `syncfs` system call allows to synchronize the filesystem containing the file pointed by
//! the given file descriptor.

use crate::errno::Errno;
use crate::file::open_file::FDTarget;
use crate::process::regs::Regs;
use crate::process::Process;

/// The implementation of the `syncfs` syscall.
pub fn syncfs(regs: &Regs) -> Result<i32, Errno> {
	let fd = regs.ebx as i32;

	if fd < 0 {
		return Err(errno!(EBADF));
	}

	let open_file_mutex = {
		let mutex = Process::get_current().unwrap();
		let guard = mutex.lock();
		let proc = guard.get_mut();

		let fd = proc.get_fd(fd as _).ok_or_else(|| errno!(EBADF))?;
		fd.get_open_file()
	};

	let open_file_guard = open_file_mutex.lock();
	let open_file = open_file_guard.get();

	match open_file.get_target() {
		FDTarget::File(f) => {
			let file_guard = f.lock();
			let file = file_guard.get();

			let location = file.get_location();
			let _mountpoint = location.get_mountpoint();

			// TODO Sync all files on mountpoint

			Ok(0)
		}

		_ => Ok(0),
	}
}
