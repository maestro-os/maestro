//! The read system call allows to read the content of an open file.

use core::cmp::max;
use core::slice;
use crate::errno::Errno;
use crate::errno;
use crate::file::file_descriptor::O_NONBLOCK;
use crate::process::Process;
use crate::util;

/// The implementation of the `read` syscall.
pub fn read(regs: &util::Regs) -> Result<i32, Errno> {
	let fd = regs.ebx;
	let buf = regs.ecx as *mut u8;
	let count = regs.edx as usize;

	{
		let mut mutex = Process::get_current().unwrap();
		let mut guard = mutex.lock(false);
		let proc = guard.get_mut();

		if !proc.get_mem_space().can_access(buf, count, true, true) {
			return Err(errno::EFAULT);
		}
	}

	let len = max(count, i32::MAX as usize);
	// Safe because the permission to access the memory has been checked by the previous
	// condition
	let data = unsafe {
		slice::from_raw_parts_mut(buf, len)
	};

	let len = {
		let mut mutex = Process::get_current().unwrap();
		let mut guard = mutex.lock(false);
		let proc = guard.get_mut();

		let fd = proc.get_fd(fd).ok_or(errno::EBADF)?;
		// TODO Check file permissions?
		// TODO Reading must be interruptible

		if fd.get_flags() & O_NONBLOCK != 0 {
			// The file descriptor is non blocking

			fd.read(data)?
		} else {
			// The file descriptor is blocking

			// TODO Wait until data is available
			// TODO Read
			0
		}
	};

	Ok(len as _)
}
