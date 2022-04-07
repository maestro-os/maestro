//! The `set_tid_address` system call sets the `clear_child_tid` attribute with the given pointer.

use core::ptr::NonNull;
use crate::errno::Errno;
use crate::process::Process;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::process::regs::Regs;

/// The implementation of the `set_tid_address` syscall.
pub fn set_tid_address(regs: &Regs) -> Result<i32, Errno> {
	let tidptr: SyscallPtr<i32> = (regs.ebx as usize).into();

	// Getting process
	let mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock();
	let proc = guard.get_mut();

	let mem_space = proc.get_mem_space().unwrap();
	let mem_space_guard = mem_space.lock();

	let ptr = NonNull::new(tidptr.as_ptr_mut());
	proc.set_clear_child_tid(ptr);

	let tid = proc.get_tid();

	// Setting the TID at pointer if accessible
	if let Some(tidptr) = tidptr.get_mut(&mem_space_guard)? {
		*tidptr = tid as _;
	}

	Ok(tid as _)
}
