//! This module implements the `write` system call, which allows to write data to a file.

use core::cmp::max;
use core::slice;
use crate::errno::Errno;
use crate::errno;
use crate::file::file_descriptor::O_NONBLOCK;
use crate::process::Process;
use crate::util;

// TODO Return EPIPE and kill with SIGPIPE when writing on a broken pipe

/// The implementation of the `write` syscall.
pub fn write(regs: &util::Regs) -> Result<i32, Errno> {
	let fd = regs.ebx;
	let buf = regs.ecx as *const u8;
	let count = regs.edx as usize;

	{
		let mut mutex = Process::get_current().unwrap();
		let mut guard = mutex.lock(false);
		let proc = guard.get_mut();

		if !proc.get_mem_space().can_access(buf, count, true, false) {
			return Err(errno::EFAULT);
		}
	}

	let len = max(count, i32::MAX as usize);
	// Safe because the permission to access the memory has been checked by the previous
	// condition
	let data = unsafe {
		slice::from_raw_parts(buf, len)
	};

	let len = {
		let mut mutex = Process::get_current().unwrap();
		let mut guard = mutex.lock(false);
		let proc = guard.get_mut();

		let fd = proc.get_fd(fd).ok_or(errno::EBADF)?;
		// TODO Check file permissions?
		// TODO Writing must be interruptible

		if fd.get_flags() & O_NONBLOCK != 0 {
			// The file descriptor is non blocking

			// TODO If blocking, EAGAIN
			fd.write(data)?
		} else {
			// The file descriptor is blocking

			// TODO Wait until able to write
			// TODO Write
			0
		}
	};

	Ok(len as _)
}
