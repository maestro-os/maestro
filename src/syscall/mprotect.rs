//! The `mprotect` system call allows to set permissions for the given range of memory.

use super::mmap;
use crate::{
	errno::Errno,
	memory,
	process::{mem_space, Process},
};
use core::ffi::{c_int, c_void};
use macros::syscall;

/// Converts the given `prot` to mapping flags.
fn prot_to_flags(prot: i32) -> u8 {
	let mut mem_flags = 0;

	if prot & mmap::PROT_WRITE != 0 {
		mem_flags |= mem_space::MAPPING_FLAG_WRITE;
	}
	if prot & mmap::PROT_EXEC != 0 {
		mem_flags |= mem_space::MAPPING_FLAG_EXEC;
	}

	mem_flags
}

#[syscall]
pub fn mprotect(addr: *mut c_void, len: usize, prot: c_int) -> Result<i32, Errno> {
	// Checking alignment of `addr` and `length`
	if !addr.is_aligned_to(memory::PAGE_SIZE) || len == 0 {
		return Err(errno!(EINVAL));
	}
	let flags = prot_to_flags(prot);

	let (mem_space_mutex, ap) = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();
		let mem_space = proc.get_mem_space().unwrap().clone();

		(mem_space, proc.access_profile)
	};
	let mut mem_space = mem_space_mutex.lock();
	mem_space.set_prot(addr, len, flags, &ap)?;

	Ok(0)
}
