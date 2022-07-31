//! TODO doc

use crate::errno::Errno;
use crate::process::regs::Regs;
use core::ffi::c_void;

/// The implementation of the `madvise` syscall.
pub fn madvise(regs: &Regs) -> Result<i32, Errno> {
	let _addr = regs.ebx as *mut c_void;
	let _length = regs.ecx as usize;
	let _advice = regs.edx as i32;

	// TODO
	Ok(0)
}
