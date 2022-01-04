//! The read system call allows to read the content of an open file.

use core::cmp::min;
use core::slice;
use crate::errno::Errno;
use crate::errno;
use crate::file::file_descriptor::O_NONBLOCK;
use crate::process::Process;
use crate::process::Regs;

// TODO O_ASYNC

/// The implementation of the `read` syscall.
pub fn read(regs: &Regs) -> Result<i32, Errno> {
	let fd = regs.ebx;
	let buf = regs.ecx as *mut u8;
	let count = regs.edx as usize;

	{
		let mutex = Process::get_current().unwrap();
		let mut guard = mutex.lock();
		let proc = guard.get_mut();

		if !proc.get_mem_space().unwrap().can_access(buf, count, true, true) {
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
		slice::from_raw_parts_mut(buf, len)
	};

	loop {
		let (len, flags) = {
			let mutex = Process::get_current().unwrap();
			let mut guard = mutex.lock();
			let proc = guard.get_mut();

			let fd = proc.get_fd(fd).ok_or(errno::EBADF)?;
			// TODO Check file permissions?

			let flags = fd.get_flags();
			(fd.read(data)?, flags)
		};

		if len > 0 || flags & O_NONBLOCK != 0 {
			// The file descriptor is non blocking
			return Ok(len as _);
		}

		// TODO Mark the process as Sleeping and wake it up when data is available?
		crate::wait();
	}
}
