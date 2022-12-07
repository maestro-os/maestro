//! The `syncfs` system call allows to synchronize the filesystem containing the
//! file pointed by the given file descriptor.

use crate::errno::Errno;
use crate::process::Process;
use core::ffi::c_int;
use macros::syscall;

/// The implementation of the `syncfs` syscall.
#[syscall]
pub fn syncfs(fd: c_int) -> Result<i32, Errno> {
	if fd < 0 {
		return Err(errno!(EBADF));
	}

	let open_file_mutex = {
		let mutex = Process::get_current().unwrap();
		let guard = mutex.lock();
		let proc = guard.get_mut();

		let fds_mutex = proc.get_fds().unwrap();
		let fds_guard = fds_mutex.lock();
		let fds = fds_guard.get();

		let fd = fds.get_fd(fd as _).ok_or_else(|| errno!(EBADF))?;
		fd.get_open_file()?
	};

	let open_file_guard = open_file_mutex.lock();
	let open_file = open_file_guard.get();

	let file_mutex = open_file.get_file()?;
	let file_guard = file_mutex.lock();
	let file = file_guard.get();

	let location = file.get_location();
	let _mountpoint = location.get_mountpoint();

	// TODO Sync all files on mountpoint

	Ok(0)
}
