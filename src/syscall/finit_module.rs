//! The `finit_module` system call allows to load a module on the kernel.

use crate::errno;
use crate::errno::Errno;
use crate::memory::malloc;
use crate::module;
use crate::module::Module;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::Process;
use crate::util::io::IO;
use core::ffi::c_int;
use macros::syscall;

#[syscall]
pub fn finit_module(fd: c_int, _param_values: SyscallString, _flags: c_int) -> Result<i32, Errno> {
	if fd < 0 {
		return Err(errno!(EBADF));
	}

	let image = {
		let open_file_mutex = {
			let proc_mutex = Process::current_assert();
			let proc = proc_mutex.lock();

			if proc.uid != 0 {
				return Err(errno!(EPERM));
			}

			let fds_mutex = proc.get_fds().unwrap();
			let fds = fds_mutex.lock();

			fds.get_fd(fd as _)
				.ok_or_else(|| errno!(EBADF))?
				.get_open_file()?
		};
		let mut open_file = open_file_mutex.lock();

		let len = open_file.get_size(); // TODO Error if file is too large for 32bit?
		let mut image = malloc::Alloc::new_default(len as usize)?;
		open_file.read(0, image.as_slice_mut())?;

		image
	};

	let module = Module::load(image.as_slice())?;
	if !module::is_loaded(module.get_name()) {
		module::add(module)?;
		Ok(0)
	} else {
		Err(errno!(EEXIST))
	}
}
