//! The `init_module` system call allows to load a module on the kernel.

use crate::errno::Errno;
use crate::errno;
use crate::module::Module;
use crate::module;
use crate::process::Process;
use crate::process::mem_space::ptr::SyscallSlice;
use crate::process::regs::Regs;

/// The implementation of the `init_module` syscall.
pub fn init_module(regs: &Regs) -> Result<i32, Errno> {
	let module_image: SyscallSlice<u8> = (regs.ebx as usize).into();
	let len = regs.ecx;

	let module = {
		let proc_mutex = Process::get_current().unwrap();
		let proc_guard = proc_mutex.lock();
		let proc = proc_guard.get();

		if proc.get_uid() != 0 {
			return Err(errno!(EPERM));
		}

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();
		let image = module_image.get(&mem_space_guard, len as usize)?.ok_or(errno!(EFAULT))?;

		Module::load(image)?
	};

	if !module::is_loaded(module.get_name()) {
		module::add(module)?;
		Ok(0)
	} else {
		Err(errno!(EEXIST))
	}
}
