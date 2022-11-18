//! The `init_module` system call allows to load a module on the kernel.

use crate::process::mem_space::ptr::SyscallString;
use core::ffi::c_ulong;
use crate::errno;
use crate::errno::Errno;
use crate::module;
use crate::module::Module;
use crate::process::mem_space::ptr::SyscallSlice;
use crate::process::Process;
use macros::syscall;

/// The implementation of the `init_module` syscall.
#[syscall]
pub fn init_module(module_image: SyscallSlice<u8>, len: c_ulong, param_values: SyscallString) -> Result<i32, Errno> {
	let module = {
		let proc_mutex = Process::get_current().unwrap();
		let proc_guard = proc_mutex.lock();
		let proc = proc_guard.get();

		if proc.get_uid() != 0 {
			return Err(errno!(EPERM));
		}

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();
		let image = module_image
			.get(&mem_space_guard, len as usize)?
			.ok_or_else(|| errno!(EFAULT))?;

		Module::load(image)?
	};

	if !module::is_loaded(module.get_name()) {
		module::add(module)?;
		Ok(0)
	} else {
		Err(errno!(EEXIST))
	}
}
