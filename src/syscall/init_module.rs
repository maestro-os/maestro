//! The `init_module` system call allows to load a module on the kernel.

use core::slice;
use crate::errno::Errno;
use crate::errno;
use crate::module::Module;
use crate::module;
use crate::process::Process;
use crate::process::Regs;

/// The implementation of the `init_module` syscall.
pub fn init_module(regs: &Regs) -> Result<i32, Errno> {
	let module_image = regs.ebx as *const u8;
	let len = regs.ecx;

	{
		let proc_mutex = Process::get_current().unwrap();
		let proc_guard = proc_mutex.lock();
		let proc = proc_guard.get();

		if proc.get_uid() != 0 {
			return Err(errno::EPERM);
		}

		if !proc.get_mem_space().unwrap().can_access(module_image, len as _, true, true) {
			return Err(errno::EFAULT);
		}
	}

	let image = unsafe { // Safe because the pointer is checked before
		slice::from_raw_parts(module_image, len as usize)
	};

	let module = Module::load(image)?;
	if !module::is_loaded(module.get_name()) {
		module::add(module)?;
		Ok(0)
	} else {
		Err(errno::EEXIST)
	}
}
