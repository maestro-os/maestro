//! The `delete_module` system call allows to unload a module from the kernel.

use core::ffi::c_uint;
use crate::errno::Errno;
use crate::errno;
use crate::module;
use crate::process::Process;
use crate::process::mem_space::ptr::SyscallString;
use crate::util::container::string::String;
use macros::syscall;

// TODO handle flags

#[syscall]
pub fn delete_module(name: SyscallString, _flags: c_uint) -> Result<i32, Errno> {
	let name = {
		let proc_mutex = Process::get_current().unwrap();
		let proc_guard = proc_mutex.lock();
		let proc = proc_guard.get();

		if proc.get_euid() != 0 {
			return Err(errno!(EPERM));
		}

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();

		let name = name.get(&mem_space_guard)?.ok_or_else(|| errno!(EFAULT))?;

		String::try_from(name)?
	};

	// TODO handle dependency (don't unload a module that is required by another)
	module::remove(&name);

	Ok(0)
}
