//! The pipe system call allows to create a pipe.

use core::ffi::c_int;
use crate::errno::Errno;
use crate::file::buffer::pipe::PipeBuffer;
use crate::file::buffer;
use crate::file::open_file;
use crate::process::Process;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::util::FailableDefault;
use crate::util::ptr::SharedPtr;
use macros::syscall;

/// The implementation of the `pipe` syscall.
#[syscall]
pub fn pipe(pipefd: SyscallPtr<[c_int; 2]>) -> Result<i32, Errno> {
	let mutex = Process::get_current().unwrap();
	let guard = mutex.lock();
	let proc = guard.get_mut();

	let mem_space = proc.get_mem_space().unwrap();
	let mem_space_guard = mem_space.lock();
	let pipefd_slice = pipefd.get_mut(&mem_space_guard)?.ok_or(errno!(EFAULT))?;

	let fds_mutex = proc.get_fds().unwrap();
	let fds_guard = fds_mutex.lock();
	let fds = fds_guard.get_mut();

	// Create pipe
	let loc = buffer::register(None, SharedPtr::new(PipeBuffer::failable_default()?)?)?;
	open_file::OpenFile::new(loc.clone(), open_file::O_RDWR)?;

	let fd0 = fds.create_fd(loc.clone(), open_file::O_RDONLY)?;
	pipefd_slice[0] = fd0.get_id() as _;

	let fd1 = fds.create_fd(loc, open_file::O_WRONLY)?;
	pipefd_slice[1] = fd1.get_id() as _;

	Ok(0)
}
