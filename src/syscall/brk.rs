//! The `brk` system call allows to displace the end of the data segment of the process, thus
//! allowing memory allocations.

use core::ffi::c_void;
use crate::errno::Errno;
use crate::process::Process;
use crate::process::regs::Regs;

/// The implementation of the `brk` syscall.
pub fn brk(regs: &Regs) -> Result<i32, Errno> {
	let addr = regs.ebx as *mut c_void;

	let mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock();
	let proc = guard.get_mut();

	let mem_space_guard = proc.get_mem_space().unwrap().lock();
	let mem_space = mem_space_guard.get();
	let old = mem_space.get_brk_ptr();

	if mem_space.set_brk_ptr(addr).is_ok() {
		Ok(addr as _)
	} else {
		Ok(old as _)
	}
}
