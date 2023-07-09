//! The `fsync` system call synchronizes the state of a file to storage.

use crate::errno;
use crate::errno::Errno;
use crate::process::Process;
use core::ffi::c_int;
use macros::syscall;

#[syscall]
pub fn fsync(fd: c_int) -> Result<i32, Errno> {
	if fd < 0 {
		return Err(errno!(EBADF));
	}

	let file_mutex = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let fds_mutex = proc.get_fds().unwrap();
		let fds = fds_mutex.lock();

		let fd = fds.get_fd(fd as _).ok_or_else(|| errno!(EBADF))?;

		let open_file_mutex = fd.get_open_file()?;
		let open_file = open_file_mutex.lock();

		open_file.get_file()?
	};

	let file = file_mutex.lock();
	file.sync()?;

	Ok(0)
}
