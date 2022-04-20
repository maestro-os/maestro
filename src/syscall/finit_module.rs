//! The `finit_module` system call allows to load a module on the kernel.

use crate::errno::Errno;
use crate::errno;
use crate::memory::malloc;
use crate::module::Module;
use crate::module;
use crate::process::Process;
use crate::process::regs::Regs;

/// The implementation of the `finit_module` syscall.
pub fn finit_module(regs: &Regs) -> Result<i32, Errno> {
	let fd = regs.ebx as u32;

	let image = {
		let proc_mutex = Process::get_current().unwrap();
		let mut proc_guard = proc_mutex.lock();
		let proc = proc_guard.get_mut();

		if proc.get_uid() != 0 {
			return Err(errno!(EPERM));
		}

		let open_file_mutex = proc.get_open_file(fd).ok_or_else(|| errno!(EBADF))?;
		let mut open_file_guard = open_file_mutex.lock();
		let open_file = open_file_guard.get_mut();

		let len = open_file.get_file_size(); // TODO Error if file is too large for 32bit?
		let mut image = unsafe {
			malloc::Alloc::new_zero(len as usize)?
		};
		open_file.read(image.as_slice_mut())?;

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
