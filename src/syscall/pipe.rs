//! The pipe system call allows to create a pipe.

use core::mem::size_of;
use crate::errno::Errno;
use crate::errno;
use crate::file::file_descriptor::FDTarget;
use crate::file::file_descriptor;
use crate::file::pipe::Pipe;
use crate::process::Process;
use crate::process::Regs;
use crate::util::ptr::SharedPtr;

/// The implementation of the `pipe` syscall.
pub fn pipe(regs: &Regs) -> Result<i32, Errno> {
	let pipefd = regs.ebx as *mut [i32; 2];

	let (fd0, fd1) = {
		let mutex = Process::get_current().unwrap();
		let mut guard = mutex.lock();
		let proc = guard.get_mut();

		let len = size_of::<i32>() * 2;
		if !proc.get_mem_space().unwrap().can_access(pipefd as _, len, true, true) {
			return Err(errno::EFAULT);
		}

		let pipe = SharedPtr::new(Pipe::new()?)?;
		let pipe2 = pipe.clone();
		let fd0 = proc.create_fd(file_descriptor::O_RDONLY, FDTarget::Pipe(pipe))?.get_id();
		let fd1 = proc.create_fd(file_descriptor::O_WRONLY, FDTarget::Pipe(pipe2))?.get_id();

		(fd0, fd1)
	};

	unsafe { // Safe because the address has been check before
		(*pipefd)[0] = fd0 as _;
		(*pipefd)[1] = fd1 as _;
	}
	Ok(0)
}
