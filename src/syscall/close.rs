//! The `close` system call closes the given file descriptor.

use core::ffi::c_int;
use crate::errno;
use crate::errno::Errno;
use crate::process::Process;
use macros::syscall;

/// The implementation of the `close` syscall.
#[syscall]
pub fn close(fd: c_int) -> Result<i32, Errno> {
	let mutex = Process::get_current().unwrap();
	let guard = mutex.lock();
	let proc = guard.get_mut();

	if proc.close_fd(fd).is_ok() {
		Ok(0)
	} else {
		Err(errno!(EBADF))
	}
}
