//! The `dup` syscall allows to duplicate a file descriptor.

use crate::errno::Errno;
use crate::file::fd::NewFDConstraint;
use crate::process::Process;
use core::ffi::c_int;
use macros::syscall;

#[syscall]
pub fn dup(oldfd: c_int) -> Result<i32, Errno> {
	if oldfd < 0 {
		return Err(errno!(EBADF));
	}

	let proc_mutex = Process::current_assert();
	let proc = proc_mutex.lock();

	let fds_mutex = proc.get_fds().unwrap();
	let mut fds = fds_mutex.lock();

	let newfd = fds.duplicate_fd(oldfd as _, NewFDConstraint::None, false)?;
	Ok(newfd.get_id() as _)
}
