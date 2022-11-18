//! The `dup` syscall allows to duplicate a file descriptor.

use core::ffi::c_int;
use crate::errno::Errno;
use crate::file::fd::NewFDConstraint;
use crate::process::Process;
use macros::syscall;

/// The implementation of the `dup` syscall.
#[syscall]
pub fn dup(oldfd: c_int) -> Result<i32, Errno> {
	let mutex = Process::get_current().unwrap();
	let guard = mutex.lock();
	let proc = guard.get_mut();

	let newfd = proc.duplicate_fd(oldfd, NewFDConstraint::None, false)?;
	Ok(newfd.get_id() as _)
}
