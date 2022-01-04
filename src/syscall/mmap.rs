//! The `mmap` system call allows the process to allocate memory.

use core::ffi::c_void;
use core::intrinsics::wrapping_add;
use crate::errno::Errno;
use crate::errno;
use crate::memory;
use crate::process::Process;
use crate::process::Regs;
use crate::process::mem_space;
use crate::util;

/// Data can be read.
const PROT_READ: i32 = 0b001;
/// Data can be written.
const PROT_WRITE: i32 = 0b010;
/// Data can be executed.
const PROT_EXEC: i32 = 0b100;

/// Changes are shared.
const MAP_SHARED: i32 = 0b001;
/// Interpret addr exactly.
const MAP_FIXED: i32 = 0b010;

/// Converts mmap's `flags` and `prot` to mem space mapping flags.
fn get_flags(flags: i32, prot: i32) -> u8 {
	let mut mem_flags = mem_space::MAPPING_FLAG_USER;

	if flags & MAP_SHARED != 0 {
		mem_flags |= mem_space::MAPPING_FLAG_SHARED;
	}

	if prot & PROT_WRITE != 0 {
		mem_flags |= mem_space::MAPPING_FLAG_WRITE;
	}
	if prot & PROT_EXEC != 0 {
		mem_flags |= mem_space::MAPPING_FLAG_EXEC;
	}

	mem_flags
}

/// The implementation of the `mmap` syscall.
pub fn mmap(regs: &Regs) -> Result<i32, Errno> {
	let addr = regs.ebx as *mut c_void;
	let length = regs.ecx as usize;
	let prot = regs.edx as i32;
	let flags = regs.esi as i32;
	let fd = regs.edi as i32;
	let offset = regs.ebp as u32;

	// Checking alignment of `addr` and `length`
	if !util::is_aligned(addr, memory::PAGE_SIZE) || length % memory::PAGE_SIZE != 0 {
		return Err(errno::EINVAL);
	}

	// The length in number of pages
	let pages = length / memory::PAGE_SIZE;

	// Checking for overflow
	let end = wrapping_add(addr as usize, length);
	if end < addr as usize {
		return Err(errno::EINVAL);
	}

	let addr_hint = {
		if !addr.is_null()
			&& (addr as usize) < (memory::PROCESS_END as usize)
			&& end <= (memory::PROCESS_END as usize) {
			Some(addr as *const c_void)
		} else {
			None
		}
	};

	// Getting the current process
	let mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock();
	let proc = guard.get_mut();

	// The file descriptor used by the mapping
	let fd = {
		if fd >= 0 {
			if let Some(fd) = proc.get_fd(fd as _) {
				Some(fd.clone())
			} else {
				None
			}
		} else {
			None
		}
	};

	if let Some(_fd) = &fd {
		// Checking the alignment of the offset
		if offset as usize % memory::PAGE_SIZE != 0 {
			return Err(errno::EINVAL);
		}

		// TODO Check the read/write state of the fd matches the mapping
	} else {
		// TODO If the mapping requires a fd, return an error
	}

	// The process's memory space
	let mem_space = proc.get_mem_space_mut().unwrap();
	// FIXME Passing the hint as an exact location
	// The pointer on the virtual memory to the beginning of the mapping
	let ptr = mem_space.map(addr_hint, pages, get_flags(flags, prot), fd, offset as _)?;
	Ok(ptr as _)
}
