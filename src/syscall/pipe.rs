//! The pipe system call allows to create a pipe.

use core::mem::size_of;
use crate::errno::Errno;
use crate::errno;
use crate::file::file_descriptor::FDTarget;
use crate::file::pipe::Pipe;
use crate::process::Process;
use crate::util::ptr::SharedPtr;
use crate::util;

/// The implementation of the `pipe` syscall.
pub fn pipe(regs: &util::Regs) -> Result<i32, Errno> {
	let pipefd = regs.ebx as *mut [i32; 2];

	let mut mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock(false);
	let proc = guard.get_mut();

	if !proc.get_mem_space().can_access(pipefd as _, size_of::<i32>() * 2, true, true) {
		return Err(errno::EFAULT);
	}

	let pipe = SharedPtr::new(Pipe::new()?);
	let fd0 = proc.create_fd(FDTarget::FileDescriptor(pipe.clone()?))?.get_id();
	let fd1 = proc.create_fd(FDTarget::FileDescriptor(pipe.clone()?))?.get_id();

	unsafe { // Safe because the address has been check before
		(*pipefd)[0] = fd0 as _;
		(*pipefd)[1] = fd1 as _;
	}
	Ok(0)
}
