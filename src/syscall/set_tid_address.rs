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
	let proc_mutex = Process::current_assert();
	let mut proc = proc_mutex.lock();

	let mem_space = proc.get_mem_space().unwrap();
	let mut mem_space_guard = mem_space.lock();

	let ptr = NonNull::new(tidptr.as_ptr_mut());
	proc.set_clear_child_tid(ptr);

	// Setting the TID at pointer if accessible
	if let Some(tidptr) = tidptr.get_mut(&mut mem_space_guard)? {
		*tidptr = proc.tid as _;
	}

	Ok(proc.tid as _)
}
