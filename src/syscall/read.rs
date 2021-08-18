//! The read system call allows to read the content of an open file.

use core::cmp::max;
use core::slice;
use crate::errno::Errno;
use crate::errno;
use crate::process::Process;
use crate::util;

// TODO Implement blocking read

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

	let len = max(count as i32, 0);
	// Safe because the permission to access the memory has been checked by the previous
	// condition
	let data = unsafe {
		slice::from_raw_parts_mut(buf, len as usize)
	};

	let len = {
		let mut mutex = Process::get_current().unwrap();
		let mut guard = mutex.lock(false);
		let proc = guard.get_mut();

		let fd = proc.get_fd(fd).ok_or(errno::EBADF)?;
		// TODO Check file permissions?

		fd.read(data)? // TODO Reading must be interruptible
	};

	Ok(len as _) // TODO Take into account when length is overflowing
}
