//! The `close` system call closes the given file descriptor.

use crate::errno;
use crate::errno::Errno;
use crate::process::Process;
use core::ffi::c_int;
use macros::syscall;

/// The implementation of the `close` syscall.
#[syscall]
pub fn close(fd: c_int) -> Result<i32, Errno> {
	if fd < 0 {
		return Err(errno!(EBADF));
	}

	let mutex = Process::get_current().unwrap();
	let guard = mutex.lock();
	let proc = guard.get_mut();

	let fds_mutex = proc.get_fds().unwrap();
	let fds_guard = fds_mutex.lock();
	let fds = fds_guard.get_mut();

	fds.close_fd(fd as _)?;
	Ok(0)
}
