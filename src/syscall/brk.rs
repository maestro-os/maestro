//! The `brk` system call allows to displace the end of the data segment of the
//! process, thus allowing memory allocations.

use crate::errno::Errno;
use crate::process::regs::Regs;
use crate::process::Process;
use core::ffi::c_void;

pub fn brk(regs: &Regs) -> Result<i32, Errno> {
	let addr = regs.ebx as *mut c_void;

	let proc_mutex = Process::current_assert();
	let proc = proc_mutex.lock();

	let mem_space = proc.get_mem_space().unwrap();
	let mut mem_space = mem_space.lock();

	let old = mem_space.get_brk_ptr();

	if mem_space.set_brk_ptr(addr).is_ok() {
		Ok(addr as _)
	} else {
		Ok(old as _)
	}
}
