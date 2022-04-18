//! The pipe system call allows to create a pipe.

use crate::errno::Errno;
use crate::file::file_descriptor::FDTarget;
use crate::file::file_descriptor;
use crate::file::pipe::Pipe;
use crate::process::Process;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::process::regs::Regs;
use crate::util::ptr::SharedPtr;

/// The implementation of the `pipe` syscall.
pub fn pipe(regs: &Regs) -> Result<i32, Errno> {
	let pipefd: SyscallPtr<[i32; 2]> = (regs.ebx as usize).into();

	let mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock();
	let proc = guard.get_mut();

	let mem_space = proc.get_mem_space().unwrap();
	let mem_space_guard = mem_space.lock();
	let pipefd_slice = pipefd.get_mut(&mem_space_guard)?.ok_or(errno!(EFAULT))?;

	let pipe = SharedPtr::new(Pipe::new(0, 0)?)?; // File descriptors are set after being created
	let (fd0_id, _) = proc.create_fd(file_descriptor::O_RDONLY, FDTarget::Pipe(pipe.clone()))?;
	let (fd1_id, _) = proc.create_fd(file_descriptor::O_WRONLY, FDTarget::Pipe(pipe.clone()))?;

	// Setting file descriptors on the pipe
	{
		let mut guard = pipe.lock();
		guard.get_mut().set_fd0(fd0_id);
		guard.get_mut().set_fd1(fd1_id);
	}

	pipefd_slice[0] = fd0_id as _;
	pipefd_slice[1] = fd1_id as _;
	Ok(0)
}
