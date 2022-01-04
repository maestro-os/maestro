//! The `delete_module` system call allows to unload a module from the kernel.

use crate::errno::Errno;
use crate::errno;
use crate::process::Process;
use crate::process::Regs;

/// The implementation of the `delete_module` syscall.
pub fn delete_module(regs: &Regs) -> Result<i32, Errno> {
	let _name = regs.ebx as *const u8;

	{
		let proc_mutex = Process::get_current().unwrap();
		let proc_guard = proc_mutex.lock();
		let proc = proc_guard.get();

		if proc.get_uid() != 0 {
			return Err(errno::EPERM);
		}

		// TODO Check the name is accessible to the process
	}

	// TODO Turn the name into a string
	// TODO Get the module with the given name
	// TODO If the module doesn't exist, return an error
	// TODO If the module exists, call its `fini` function if it exists, then unload the module
	Ok(0)
}
