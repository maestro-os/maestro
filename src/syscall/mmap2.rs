//! The `mmap2` system call is similar to `mmap`, except it takes a file offset in pages.

use core::ffi::c_void;
use crate::errno::Errno;
use crate::process::Regs;
use super::mmap;

/// The implementation of the `mmap2` syscall.
pub fn mmap2(regs: &Regs) -> Result<i32, Errno> {
	let addr = regs.ebx as *mut c_void;
	let length = regs.ecx as usize;
	let prot = regs.edx as i32;
	let flags = regs.esi as i32;
	let fd = regs.edi as i32;
	let offset = regs.ebp as u32;

	mmap::do_mmap(addr, length, prot, flags, fd, (offset as u64) * 4096)
}
