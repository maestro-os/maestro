//! The pipe2 system call allows to create a pipe with given flags.

use core::ffi::c_int;
use crate::errno;
use crate::errno::Errno;
use crate::file::open_file;
use crate::file::open_file::FDTarget;
use crate::file::pipe::PipeBuffer;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::process::Process;
use crate::util::ptr::SharedPtr;

/// The implementation of the `pipe2` syscall.
pub fn pipe2(pipefd: SyscallPtr<[c_int; 2]>, flags: c_int) -> Result<i32, Errno> {
	let accepted_flags = open_file::O_CLOEXEC | open_file::O_DIRECT | open_file::O_NONBLOCK;
	if flags & !accepted_flags != 0 {
		return Err(errno!(EINVAL));
	}

	let mutex = Process::get_current().unwrap();
	let guard = mutex.lock();
	let proc = guard.get_mut();

	let mem_space = proc.get_mem_space().unwrap();
	let mem_space_guard = mem_space.lock();
	let pipefd_slice = pipefd.get_mut(&mem_space_guard)?.ok_or(errno!(EFAULT))?;

	let pipe = SharedPtr::new(PipeBuffer::new()?)?;
	let fd0 = proc.create_fd(open_file::O_RDONLY | flags, FDTarget::Pipe(pipe.clone()))?;
	let fd1 = proc.create_fd(open_file::O_WRONLY | flags, FDTarget::Pipe(pipe.clone()))?;

	pipefd_slice[0] = fd0.get_id() as _;
	pipefd_slice[1] = fd1.get_id() as _;
	Ok(0)
}
