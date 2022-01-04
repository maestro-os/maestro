//! The `set_tid_address` system call sets the `clear_child_tid` attribute with the given pointer.

use core::ptr::NonNull;
use crate::errno::Errno;
use crate::process::Process;
use crate::process::regs::Regs;

/// The implementation of the `set_tid_address` syscall.
pub fn set_tid_address(regs: &Regs) -> Result<i32, Errno> {
	let tidptr = regs.ebx as *mut i32;

	// Getting process
	let mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock();
	let proc = guard.get_mut();

	let ptr = NonNull::new(tidptr);
	proc.set_clear_child_tid(ptr);

	// TODO Write TID at address? (if accessible)

	Ok(proc.get_tid() as _)
}
