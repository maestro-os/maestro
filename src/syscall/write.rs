//! This module implements the `write` system call, which allows to write data to a file.

use core::slice;
use core::str;
use crate::errno::Errno;
use crate::errno;
use crate::process::Process;
use crate::util;

/// The implementation of the `write` syscall.
pub fn write(proc: &mut Process, regs: &util::Regs) -> Result<i32, Errno> {
	let _fd = regs.ebx;
	let buf = regs.ecx as *const u8;
	let count = regs.edx as usize;

	if proc.get_mem_space().can_access(buf, count, true, true) {
		// Safe because the permission to access the memory has been checked by the previous
		// condition
		let data = str::from_utf8(unsafe {
			slice::from_raw_parts(buf, count)
		}).unwrap();

		// TODO Write into the file
		crate::print!("{}", data);

		Ok(0)
	} else {
		Err(errno::EFAULT)
	}
}
