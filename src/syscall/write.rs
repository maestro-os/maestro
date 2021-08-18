//! This module implements the `write` system call, which allows to write data to a file.

use core::cmp::max;
use core::slice;
use crate::errno::Errno;
use crate::errno;
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

	let len = max(count as i32, 0);
	// Safe because the permission to access the memory has been checked by the previous
	// condition
	let data = unsafe {
		slice::from_raw_parts(buf, len as usize)
	};

	let len = {
		let mut mutex = Process::get_current().unwrap();
		let mut guard = mutex.lock(false);
		let proc = guard.get_mut();

		let fd = proc.get_fd(fd).ok_or(errno::EBADF)?;
		// TODO Check file permissions?

		fd.write(data)? // TODO Writing must be interruptible
	};

	Ok(len as _) // TODO Take into account when length is overflowing
}
