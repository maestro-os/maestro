//! This module implements the `write` system call, which allows to write data to a file.

use core::cmp::max;
use core::slice;
use crate::errno::Errno;
use crate::errno;
use crate::process::Process;
use crate::util::lock::mutex::TMutex;
use crate::util;

/// The implementation of the `write` syscall.
pub fn write(proc: &mut Process, regs: &util::Regs) -> Result<i32, Errno> {
	let fd = regs.ebx;
	let buf = regs.ecx as *const u8;
	let count = regs.edx as usize;

	if proc.get_mem_space().can_access(buf, count, true, false) {
		let len = max(count as i32, 0);
		// Safe because the permission to access the memory has been checked by the previous
		// condition
		let data = unsafe {
			slice::from_raw_parts(buf, len as usize)
		};

		let fd = proc.get_fd(fd).ok_or(errno::EBADF)?;
		// TODO Check file permissions?
		let off = fd.get_offset();

		let len = {
			let file = fd.get_file_mut();
			let mut file_guard = file.lock();
			file_guard.get_mut().write(off as usize, data)?
		};
		fd.set_offset(off + len as u64);

		Ok(len as _) // TODO Take into account when length is overflowing
	} else {
		Err(errno::EFAULT)
	}
}
