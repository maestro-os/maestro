//! The `syncfs` system call allows to synchronize the filesystem containing the
//! file pointed by the given file descriptor.

use crate::errno::Errno;
use crate::process::Process;
use core::ffi::c_int;
use macros::syscall;

#[syscall]
pub fn syncfs(fd: c_int) -> Result<i32, Errno> {
	if fd < 0 {
		return Err(errno!(EBADF));
	}

	let open_file_mutex = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let fds_mutex = proc.get_fds().unwrap();
		let fds = fds_mutex.lock();

		let fd = fds.get_fd(fd as _).ok_or_else(|| errno!(EBADF))?;
		fd.get_open_file()?
	};

	let open_file = open_file_mutex.lock();

	let file_mutex = open_file.get_file()?;
	let file = file_mutex.lock();

	let location = file.get_location();
	let _mountpoint = location.get_mountpoint();

	// TODO Sync all files on mountpoint

	Ok(0)
}
