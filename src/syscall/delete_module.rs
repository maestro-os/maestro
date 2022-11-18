//! The `delete_module` system call allows to unload a module from the kernel.

use core::ffi::c_uint;
use crate::errno::Errno;
use crate::errno;
use crate::process::Process;
use crate::process::mem_space::ptr::SyscallString;
use macros::syscall;

/// The implementation of the `delete_module` syscall.
#[syscall]
pub fn delete_module(_name: SyscallString, _flags: c_uint) -> Result<i32, Errno> {
	{
		let proc_mutex = Process::get_current().unwrap();
		let proc_guard = proc_mutex.lock();
		let proc = proc_guard.get();

		if proc.get_uid() != 0 {
			return Err(errno!(EPERM));
		}

		// TODO Check the name is accessible to the process
	}

	// TODO Turn the name into a string
	// TODO Get the module with the given name
	// TODO If the module doesn't exist, return an error
	// TODO If the module exists, call its `fini` function if it exists, then unload
	// the module
	Ok(0)
}
