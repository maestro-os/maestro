//! The pipe2 system call allows to create a pipe with given flags.

use core::mem::size_of;
use crate::errno::Errno;
use crate::errno;
use crate::file::file_descriptor::FDTarget;
use crate::file::file_descriptor;
use crate::file::pipe::Pipe;
use crate::process::Process;
use crate::process::Regs;
use crate::util::ptr::SharedPtr;

/// The implementation of the `pipe2` syscall.
pub fn pipe2(regs: &Regs) -> Result<i32, Errno> {
	let pipefd = regs.ebx as *mut [i32; 2];
	let flags = regs.ecx as i32;

	let accepted_flags = file_descriptor::O_CLOEXEC | file_descriptor::O_DIRECT
		| file_descriptor::O_NONBLOCK;
	if flags & !accepted_flags != 0 {
		return Err(errno::EINVAL);
	}

	let (fd0, fd1) = {
		let mutex = Process::get_current().unwrap();
		let mut guard = mutex.lock();
		let proc = guard.get_mut();

		let len = size_of::<i32>() * 2;
		if !proc.get_mem_space().unwrap().can_access(pipefd as _, len, true, true) {
			return Err(errno::EFAULT);
		}

		let pipe = SharedPtr::new(Pipe::new()?);
		let fd0 = proc.create_fd(file_descriptor::O_RDONLY | flags,
			FDTarget::Pipe(pipe.clone()?))?.get_id();
		let fd1 = proc.create_fd(file_descriptor::O_WRONLY | flags,
			FDTarget::Pipe(pipe.clone()?))?.get_id();

		(fd0, fd1)
	};

	unsafe { // Safe because the address has been check before
		(*pipefd)[0] = fd0 as _;
		(*pipefd)[1] = fd1 as _;
	}
	Ok(0)
}
