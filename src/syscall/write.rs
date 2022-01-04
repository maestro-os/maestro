//! This module implements the `write` system call, which allows to write data to a file.

use core::cmp::min;
use core::slice;
use crate::errno::Errno;
use crate::errno;
use crate::file::file_descriptor::O_NONBLOCK;
use crate::process::Process;
use crate::process::Regs;

// TODO Return EPIPE and kill with SIGPIPE when writing on a broken pipe
// TODO O_ASYNC

/// The implementation of the `write` syscall.
pub fn write(regs: &Regs) -> Result<i32, Errno> {
	let fd = regs.ebx;
	let buf = regs.ecx as *const u8;
	let count = regs.edx as usize;

	{
		let mutex = Process::get_current().unwrap();
		let mut guard = mutex.lock();
		let proc = guard.get_mut();

		if !proc.get_mem_space().unwrap().can_access(buf, count, true, false) {
			return Err(errno::EFAULT);
		}
	}

	let len = min(count, i32::MAX as usize);
	if len == 0 {
		return Ok(0);
	}

	// Safe because the permission to access the memory has been checked by the previous
	// condition
	let data = unsafe {
		slice::from_raw_parts(buf, len)
	};

	loop {
		// Trying to write and getting the length of written data
		let (len, flags) = {
			let mutex = Process::get_current().unwrap();
			let mut guard = mutex.lock();
			let proc = guard.get_mut();

			let fd = proc.get_fd(fd).ok_or(errno::EBADF)?;
			// TODO Check file permissions?

			let flags = fd.get_flags();
			(fd.write(data)?, flags)
		};

		// TODO Continue until everything was written?
		// If the length is greater than zero, success
		if len > 0 {
			return Ok(len as _);
		}

		if flags & O_NONBLOCK != 0 {
			// The file descriptor is non blocking
			return Err(errno::EAGAIN);
		}

		// TODO Mark the process as Sleeping and wake it up when data can be written?
		crate::wait();
	}
}
