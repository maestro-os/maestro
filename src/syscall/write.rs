//! This module implements the `write` system call, which allows to write data to a file.

use core::slice;
use crate::errno::Errno;
use crate::errno;
use crate::process::Process;
use crate::util;

/// The implementation of the `write` syscall.
pub fn write(proc: &mut Process, regs: &util::Regs) -> Result<i32, Errno> {
	let fd = regs.ebx;
	let buf = regs.ecx as *const u8;
	let count = regs.edx as usize;

	if proc.get_mem_space().can_access(buf, count, true, false) {
		// Safe because the permission to access the memory has been checked by the previous
		// condition
		let data = unsafe {
			slice::from_raw_parts(buf, count)
		};
		let fd = proc.get_fd(fd).ok_or(errno::EBADF)?;
		// TODO Take offset of fd
		let len = fd.get_file().write(0, data)?;
		// TODO Update offset of fd
		Ok(len as _) // TODO Take into account when length is overflowing
	} else {
		Err(errno::EFAULT)
	}
}
