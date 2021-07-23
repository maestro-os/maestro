//! The `munmap` system call allows the process to free memory that was allocated with `mmap`.

//use crate::errno;
use core::ffi::c_void;
use crate::errno::Errno;
use crate::memory;
use crate::process::Process;
use crate::util::math;
use crate::util;

// TODO Prevent unmapping kernel memory

/// The implementation of the `munmap` syscall.
pub fn munmap(regs: &util::Regs) -> Result<i32, Errno> {
	let addr = regs.ebx as *mut c_void;
	let length = regs.ecx as usize;

	let mut mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock(false);
	let proc = guard.get_mut();
	let mem_space = proc.get_mem_space_mut();

	let pages = math::ceil_division(length, memory::PAGE_SIZE);

	// TODO Check for overflow on addr + pages * PAGE_SIZE
	mem_space.unmap(addr, pages);
	Ok(0)
}
