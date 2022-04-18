//! The ioctl syscall allows to control a device represented by a file descriptor.

use core::ffi::c_void;
use crate::errno::Errno;
use crate::errno;
use crate::process::Process;
use crate::process::regs::Regs;

/// The implementation of the `ioctl` syscall.
pub fn ioctl(regs: &Regs) -> Result<i32, Errno> {
	let fd = regs.ebx as i32;
	let request = regs.ecx as u32;
	let argp = regs.edx as *const c_void;

	// Getting the process
	let mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock();
	let proc = guard.get_mut();

	// TODO Check access to args (if needed)

	// Getting the file descriptor
	let file_desc_mutex = proc.get_fd(fd as _).ok_or_else(|| errno!(EBADF))?;
	let mut file_desc_guard = file_desc_mutex.lock();
	let file_desc = file_desc_guard.get_mut();

	Ok(file_desc.ioctl(request, argp)? as _)
}
