//! The `execve` system call allows to execute a program from a file.

use core::slice;
use core::str;
use crate::errno::Errno;
use crate::errno;
use crate::file::path::Path;
use crate::file;
use crate::idt;
use crate::process::Process;
use crate::process::Regs;

/// The implementation of the `execve` syscall.
pub fn execve(regs: &Regs) -> Result<i32, Errno> {
	let pathname = regs.ebx as *const u8;
	let _argv = regs.ecx as *const () as *const *const u8;
	let _envp = regs.edx as *const () as *const *const u8;

	// Checking that parameters are accessible by the process
	let pathname_len = {
		let mut mutex = Process::get_current().unwrap();
		let mut guard = mutex.lock(false);
		let proc = guard.get_mut();

		let pathname_len = proc.get_mem_space().unwrap().can_access_string(pathname, true, false);
		if pathname_len.is_none() {
			return Err(errno::EFAULT);
		}

		// TODO Check argv and envp

		pathname_len.unwrap()
	};

	// TODO Ensure from_utf8_unchecked is safe with invalid UTF-8 (probably not)
	// The path to the executable file
	let path = Path::from_string(unsafe { // Safe because the address is checked before
		str::from_utf8_unchecked(slice::from_raw_parts(pathname, pathname_len))
	}, true)?;

	let mutex = file::get_files_cache();
	let mut guard = mutex.lock(true);
	let files_cache = guard.get_mut();

	// The file
	let _file = files_cache.get_file_from_path(&path)?;

	// TODO Look for shebang

	idt::wrap_disable_interrupts(|| {
		// TODO Execute with arguments and environment

		crate::enter_loop();
	})
}
