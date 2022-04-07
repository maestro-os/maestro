//! The pipe2 system call allows to create a pipe with given flags.

use crate::errno::Errno;
use crate::errno;
use crate::file::file_descriptor::FDTarget;
use crate::file::file_descriptor;
use crate::file::pipe::Pipe;
use crate::process::Process;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::process::regs::Regs;
use crate::util::ptr::SharedPtr;

/// The implementation of the `pipe2` syscall.
pub fn pipe2(regs: &Regs) -> Result<i32, Errno> {
	let pipefd: SyscallPtr<[i32; 2]> = (regs.ebx as usize).into();
	let flags = regs.ecx as i32;

	let accepted_flags = file_descriptor::O_CLOEXEC | file_descriptor::O_DIRECT
		| file_descriptor::O_NONBLOCK;
	if flags & !accepted_flags != 0 {
		return Err(errno!(EINVAL));
	}

	let mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock();
	let proc = guard.get_mut();

	let mem_space = proc.get_mem_space().unwrap();
	let mem_space_guard = mem_space.lock();
	let pipefd_slice = pipefd.get_mut(&mem_space_guard)?.ok_or(errno!(EFAULT))?;

	let pipe = SharedPtr::new(Pipe::new()?);
	let fd0 = proc.create_fd(file_descriptor::O_RDONLY | flags,
		FDTarget::Pipe(pipe.clone()?))?.get_id();
	let fd1 = proc.create_fd(file_descriptor::O_WRONLY | flags,
		FDTarget::Pipe(pipe.clone()?))?.get_id();

	pipefd_slice[0] = fd0 as _;
	pipefd_slice[1] = fd1 as _;
	Ok(0)
}
