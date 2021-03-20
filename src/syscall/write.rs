/// This module implements the `write` system call, which allows to write data to a file.

use core::slice;
use core::str;
use crate::util;

/// The implementation of the `write` syscall.
pub fn write(regs: &util::Regs) -> u32 {
	let _fd = regs.ebx;
	let buf = regs.ecx as *const u8;
	let count = regs.edx as usize;

	// TODO Check that buffer is accessible from process
	// TODO Write into a file
	crate::print!("{}", str::from_utf8(unsafe { // Call to unsafe function
		slice::from_raw_parts(buf, count)
	}).unwrap());

	// TODO
	0
}
