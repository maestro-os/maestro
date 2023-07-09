//! The msync system call synchronizes a memory mapping with its file on the
//! disk.

use crate::errno;
use crate::errno::Errno;
use crate::memory;
use crate::process::Process;
use core::ffi::c_int;
use core::ffi::c_void;
use macros::syscall;

/// Schedules a synchronization and returns directly.
const MS_ASYNC: i32 = 0b001;
/// Synchronizes the mapping before returning.
const MS_SYNC: i32 = 0b010;
/// Invalides other mappings of the same file so they can be updated.
const MS_INVALIDATE: i32 = 0b100;

#[syscall]
pub fn msync(addr: *mut c_void, length: usize, flags: c_int) -> Result<i32, Errno> {
	// Checking address alignment
	if !addr.is_aligned_to(memory::PAGE_SIZE) {
		return Err(errno!(EINVAL));
	}
	// Checking for conflicts in flags
	if flags & MS_ASYNC != 0 && flags & MS_SYNC != 0 {
		return Err(errno!(EINVAL));
	}

	let proc_mutex = Process::current_assert();
	let proc = proc_mutex.lock();

	// The process's memory space
	let mem_space = proc.get_mem_space().unwrap();
	let mut mem_space = mem_space.lock();

	let mut i = 0;
	while i < length {
		let mapping = mem_space.get_mapping_mut_for(addr).ok_or(errno!(ENOMEM))?;
		mapping.fs_sync()?; // TODO Use flags

		i += mapping.get_size().get() * memory::PAGE_SIZE;
	}

	Ok(0)
}
