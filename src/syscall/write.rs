/// This module implements the `write` system call, which allows to write data to a file.

use core::slice;
use core::str;
use crate::errno;
use crate::process::Process;
use crate::util::lock::mutex::MutexGuard;
use crate::util;

/// The implementation of the `write` syscall.
pub fn write(_proc: &mut Process, regs: &util::Regs) -> u32 {
	let _fd = regs.ebx;
	let buf = regs.ecx as *const u8;
	let count = regs.edx as usize;

	let mut mutex = Process::get_current().unwrap();
	let mut guard = MutexGuard::new(&mut mutex);
	let curr_proc = guard.get_mut();

	if curr_proc.get_mem_space().can_access(buf, count, true, true) {
		// Safe because the permission to access the memory has been checked by the previous
		// condition
		let data = str::from_utf8(unsafe {
			slice::from_raw_parts(buf, count)
		}).unwrap();

		// TODO Write into the file
		crate::print!("{}", data);

		0
	} else {
		-errno::EFAULT as _
	}
}
