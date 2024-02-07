//! The `munmap` system call allows the process to free memory that was
//! allocated with `mmap`.

use crate::{errno, errno::Errno, memory, process::Process};
use core::{ffi::c_void, num::NonZeroUsize};
use macros::syscall;

#[syscall]
pub fn munmap(addr: *mut c_void, length: usize) -> Result<i32, Errno> {
	if !addr.is_aligned_to(memory::PAGE_SIZE) || length == 0 {
		return Err(errno!(EINVAL));
	}

	let proc_mutex = Process::current_assert();
	let proc = proc_mutex.lock();

	let pages = length.div_ceil(memory::PAGE_SIZE);
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
		.unmap(addr, NonZeroUsize::new(pages).unwrap(), false)?;
	Ok(0)
}
