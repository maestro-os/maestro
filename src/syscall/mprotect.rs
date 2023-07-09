//! The `mprotect` system call allows to set permissions for the given range of memory.

use super::mmap;
use crate::errno::Errno;
use crate::memory;
use crate::process::mem_space;
use crate::process::Process;
use core::ffi::c_int;
use core::ffi::c_void;
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
	if addr as usize % memory::PAGE_SIZE != 0 {
		return Err(errno!(EINVAL));
	}

	let flags = prot_to_flags(prot);

	let (uid, gid, mem_space_mutex) = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let uid = proc.uid;
		let gid = proc.gid;

		let mem_space = proc.get_mem_space().unwrap();

		(uid, gid, mem_space)
	};

	let mut mem_space = mem_space_mutex.lock();
	mem_space.set_prot(addr, len, flags, uid, gid)?;

	Ok(0)
}
