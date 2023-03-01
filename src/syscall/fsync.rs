//! The `fsync` system call synchronizes the state of a file to storage.

use core::ffi::c_int;
use crate::errno::Errno;
use crate::errno;
use crate::process::Process;
use macros::syscall;

#[syscall]
pub fn fsync(fd: c_int) -> Result<i32, Errno> {
	if fd < 0 {
		return Err(errno!(EBADF));
	}

	let file_mutex = {
		let mutex = Process::get_current().unwrap();
		let guard = mutex.lock();
		let proc = guard.get_mut();

		let fds_mutex = proc.get_fds().unwrap();
		let fds_guard = fds_mutex.lock();
		let fds = fds_guard.get_mut();

		let fd = fds.get_fd(fd as _).ok_or_else(|| errno!(EBADF))?;

		let open_file_mutex = fd.get_open_file()?;
		let open_file_guard = open_file_mutex.lock();
		let open_file = open_file_guard.get();

		open_file.get_file()?
	};

	let file_guard = file_mutex.lock();
	let file = file_guard.get();
	file.sync()?;

	Ok(0)
}
