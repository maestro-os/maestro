//! The `finit_module` system call allows to load a module on the kernel.

use crate::errno::Errno;
use crate::errno;
use crate::memory::malloc;
use crate::module::Module;
use crate::module;
use crate::process::Process;
use crate::process::Regs;

/// The implementation of the `finit_module` syscall.
pub fn finit_module(regs: &Regs) -> Result<i32, Errno> {
	let fd = regs.ebx as u32;

	let image = {
		let proc_mutex = Process::get_current().unwrap();
		let mut proc_guard = proc_mutex.lock();
		let proc = proc_guard.get_mut();

		if proc.get_uid() != 0 {
			return Err(errno::EPERM);
		}

		if let Some(fd) = proc.get_fd(fd) {
			let len = fd.get_len(); // TODO Error if file is too large for 32bit?
			let mut image = unsafe {
				malloc::Alloc::new_zero(len as usize)?
			};
			fd.read(image.get_slice_mut())?;

			image
		} else {
			return Err(errno::EBADF);
		}
	};

	let module = Module::load(image.get_slice())?;
	if !module::is_loaded(module.get_name()) {
		module::add(module)?;
		Ok(0)
	} else {
		Err(errno::EEXIST)
	}
}
