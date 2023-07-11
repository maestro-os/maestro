//! The `delete_module` system call allows to unload a module from the kernel.

use crate::errno;
use crate::errno::Errno;
use crate::module;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::Process;
use crate::util::container::string::String;
use core::ffi::c_uint;
use macros::syscall;

// TODO handle flags

#[syscall]
pub fn delete_module(name: SyscallString, _flags: c_uint) -> Result<i32, Errno> {
	let name = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		if proc.euid != 0 {
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
