//! The getcwd system call allows to retrieve the current working directory of the current process.

use crate::errno::Errno;
use crate::errno;
use crate::process::Process;
use crate::process::Regs;

/// The implementation of the `getcwd` syscall.
pub fn getcwd(regs: &Regs) -> Result<i32, Errno> {
	let buf = regs.ebx as *mut u8;
	let size = regs.ecx as u32;

	if size == 0 && !buf.is_null() {
		return Err(errno::EINVAL);
	}

	let cwd = {
		let mutex = Process::get_current().unwrap();
		let mut guard = mutex.lock();
		let proc = guard.get_mut();

		// Checking that the buffer is accessible
		if !proc.get_mem_space().unwrap().can_access(buf, size as _, true, true) {
			return Err(errno::EFAULT);
		}

		proc.get_cwd().as_string()?
	};
	// Checking that the buffer is large enough
	if (size as usize) < cwd.len() + 1 {
		return Err(errno::ERANGE);
	}

	for i in 0..cwd.len() {
		unsafe { // Safe because the range is check before
			*buf.add(i) = cwd.as_bytes()[i];
		}
	}
	unsafe { // Safe because the range is check before
		*buf.add(cwd.len()) = b'\0';
	}

	Ok(buf as _)
}
