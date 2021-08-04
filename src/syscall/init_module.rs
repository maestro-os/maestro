//! The `init_module` system call allows to load a module on the kernel.

use crate::errno::Errno;
use crate::errno;
use crate::process::Process;
use crate::util;

/// The implementation of the `init_module` syscall.
pub fn init_module(regs: &util::Regs) -> Result<i32, Errno> {
	let _module_image = regs.ebx;
	let _len = regs.ecx;

	{
		let mut proc_mutex = Process::get_current().unwrap();
		let proc_guard = proc_mutex.lock(false);
		let proc = proc_guard.get();

		if proc.get_uid() != 0 {
			return Err(errno::EPERM);
		}

		// TODO Check the name is accessible to the process
	}

	// TODO Parse ELF and check module correctness
	// TODO Call module's init function
	// TODO Get the name of the module and check if another module with the same name is alreday
	// loaded
	// TODO If the name is already taken, return an error
	// TODO Else, add the module to the list
	Ok(0)
}
