//! The `munmap` system call allows the process to free memory that was
//! allocated with `mmap`.

use crate::errno;
use crate::errno::Errno;
use crate::memory;
use crate::process::Process;
use crate::util;
use crate::util::math;
use core::ffi::c_void;
use macros::syscall;

#[syscall]
pub fn munmap(addr: *mut c_void, length: usize) -> Result<i32, Errno> {
	if !util::is_aligned(addr, memory::PAGE_SIZE) || length == 0 {
		return Err(errno!(EINVAL));
	}

	let mutex = Process::get_current().unwrap();
	let guard = mutex.lock();
	let proc = guard.get_mut();

	let pages = math::ceil_div(length, memory::PAGE_SIZE);
	let length = pages * memory::PAGE_SIZE;

	// Checking for overflow
	let end = (addr as usize).wrapping_add(length);
	if end < addr as usize {
		return Err(errno!(EINVAL));
	}

	// Prevent from unmapping kernel memory
	if (addr as usize) >= (memory::PROCESS_END as usize) || end > (memory::PROCESS_END as usize) {
		return Err(errno!(EINVAL));
	}

	proc.get_mem_space()
		.unwrap()
		.lock()
		.get_mut()
		.unmap(addr, pages, false)?;
	Ok(0)
}
