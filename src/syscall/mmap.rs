//! The `mmap` system call allows the process to allocate memory.

use core::ffi::c_void;
use core::intrinsics::wrapping_add;
use crate::errno::Errno;
use crate::errno;
use crate::memory;
use crate::process::Process;
use crate::process::mem_space;
use crate::util::math;
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
pub fn mmap(regs: &util::Regs) -> Result<i32, Errno> {
	let addr = regs.ebx as *mut c_void;
	let length = regs.ecx as usize;
	let prot = regs.edx as i32;
	let flags = regs.esi as i32;
	let _fd = regs.edi as i32;
	let _offset = regs.ebp as u32;

	let mut mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock(false);
	let proc = guard.get_mut();
	let mem_space = proc.get_mem_space_mut();

	let addr = util::down_align(addr, memory::PAGE_SIZE);
	let pages = math::ceil_division(length, memory::PAGE_SIZE);
	let length = pages * memory::PAGE_SIZE;

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

	let ptr = mem_space.map(addr_hint, pages, get_flags(flags, prot))?;
	// TODO Handle fd and offset
	Ok(ptr as _)
}
