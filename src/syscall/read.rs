//! The read system call allows to read the content of an open file.

use core::cmp::min;
use crate::errno::Errno;
use crate::errno;
use crate::file::file_descriptor::O_NONBLOCK;
use crate::process::Process;
use crate::process::mem_space::ptr::SyscallSlice;
use crate::process::regs::Regs;

// TODO O_ASYNC

/// The implementation of the `read` syscall.
pub fn read(regs: &Regs) -> Result<i32, Errno> {
	let fd = regs.ebx;
	let buf: SyscallSlice<u8> = (regs.ecx as usize).into();
	let count = regs.edx as usize;

	let len = min(count, i32::MAX as usize);
	if len == 0 {
		return Ok(0);
	}

	loop {
		let (len, flags) = {
			let mutex = Process::get_current().unwrap();
			let mut guard = mutex.lock();
			let proc = guard.get_mut();

			let mem_space = proc.get_mem_space().unwrap();
			let mem_space_guard = mem_space.lock();
			let buf_slice = buf.get_mut(&mem_space_guard, len)?.ok_or(errno!(EFAULT))?;

			let file_desc_mutex = proc.get_fd(fd).ok_or(errno!(EBADF))?;
			let mut file_desc_guard = file_desc_mutex.lock();
			let file_desc = file_desc_guard.get_mut();

			if file_desc.eof() {
				return Ok(0);
			}

			let flags = file_desc.get_flags();
			(file_desc.read(buf_slice)?, flags)
		};

		if len > 0 || flags & O_NONBLOCK != 0 {
			// The file descriptor is non blocking
			return Ok(len as _);
		}

		// TODO Mark the process as Sleeping and wake it up when data is available?
		crate::wait();
	}
}
