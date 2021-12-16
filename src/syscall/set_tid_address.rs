//! TODO doc

use crate::errno::Errno;
use crate::process::regs::Regs;

/// The implementation of the `set_tid_address` syscall.
pub fn set_tid_address(regs: &Regs) -> Result<i32, Errno> {
	let _tidptr = regs.ebx as *mut i32;

	// TODO
	Ok(0)
}
