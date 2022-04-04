//! The `clone` system call creates a child process.

use core::ffi::c_void;
use crate::errno::Errno;
use crate::process::regs::Regs;

/// The implementation of the `clone` syscall.
pub fn clone(regs: &Regs) -> Result<i32, Errno> {
	let _flags = regs.ebx as i32;
	let _stack = regs.ecx as *mut c_void;
	let _parent_tid = regs.edx as *mut i32;
	let _tls = regs.esi as i32;
	let _child_tid = regs.edi as *mut i32;

	// TODO
	todo!();
}
