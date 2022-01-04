//! The msync system call synchronizes a memory mapping with its file on the disk.

use core::ffi::c_void;
use crate::errno::Errno;
use crate::errno;
use crate::memory;
use crate::process::Process;
use crate::process::Regs;
use crate::util;

/// Schedules a synchronization and returns directly.
const MS_ASYNC: i32 = 0b001;
/// Synchronizes the mapping before returning.
const MS_SYNC: i32 = 0b010;
/// Invalides other mappings of the same file so they can be updated.
const MS_INVALIDATE: i32 = 0b100;

/// The implementation of the `msync` syscall.
pub fn msync(regs: &Regs) -> Result<i32, Errno> {
	let addr = regs.ebx as *mut c_void;
	let length = regs.ecx as usize;
	let flags = regs.edx as i32;

	// Checking address alignment
	if !util::is_aligned(addr, memory::PAGE_SIZE) {
		return Err(errno::EINVAL);
	}
	// Checking for conflicts in flags
	if flags & MS_ASYNC != 0 && flags & MS_SYNC != 0 {
		return Err(errno::EINVAL);
	}

	// Getting the current process
	let mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock();
	let proc = guard.get_mut();

	// The process's memory space
	let mem_space = proc.get_mem_space_mut().unwrap();

	let mut i = 0;
	while i < length {
		let mapping = mem_space.get_mapping_mut_for(addr).ok_or(errno::ENOMEM)?;
		mapping.fs_sync()?; // TODO Use flags

		i += mapping.get_size() * memory::PAGE_SIZE;
	}

	Ok(0)
}
