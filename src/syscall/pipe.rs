//! The pipe system call allows to create a pipe.

use crate::errno::Errno;
use crate::file::open_file::FDTarget;
use crate::file::open_file;
use crate::file::pipe::PipeBuffer;
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

	let pipe = SharedPtr::new(PipeBuffer::new()?)?;
	let (fd0_id, _) = proc.create_fd(open_file::O_RDONLY, FDTarget::Pipe(pipe.clone()))?;
	let (fd1_id, _) = proc.create_fd(open_file::O_WRONLY, FDTarget::Pipe(pipe.clone()))?;

	pipefd_slice[0] = fd0_id as _;
	pipefd_slice[1] = fd1_id as _;
	Ok(0)
}
