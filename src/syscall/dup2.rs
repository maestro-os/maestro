//! The `dup2` syscall allows to duplicate a file descriptor, specifying the id
//! of the newly created file descriptor.

use crate::errno::Errno;
use crate::file::fd::NewFDConstraint;
use crate::process::Process;
use core::ffi::c_int;
use macros::syscall;

/// The implementation of the `dup2` syscall.
#[syscall]
pub fn dup2(oldfd: c_int, newfd: c_int) -> Result<i32, Errno> {
	if oldfd < 0 || newfd < 0 {
		return Err(errno!(EBADF));
	}

	let mutex = Process::get_current().unwrap();
	let guard = mutex.lock();
	let proc = guard.get_mut();

	let fds_mutex = proc.get_fds().unwrap();
	let fds_guard = fds_mutex.lock();
	let fds = fds_guard.get_mut();

	let newfd = fds.duplicate_fd(oldfd as _, NewFDConstraint::Fixed(newfd as _), false)?;
	Ok(newfd.get_id() as _)
}
