//! The `set_tid_address` system call sets the `clear_child_tid` attribute with
//! the given pointer.

use crate::errno::Errno;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::process::Process;
use core::ffi::c_int;
use core::ptr::NonNull;
use macros::syscall;

#[syscall]
pub fn set_tid_address(tidptr: SyscallPtr<c_int>) -> Result<i32, Errno> {
	// Getting process
	let mutex = Process::get_current().unwrap();
	let guard = mutex.lock();
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
