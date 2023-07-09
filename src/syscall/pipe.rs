//! The pipe system call allows to create a pipe.

use crate::errno::Errno;
use crate::file::buffer;
use crate::file::buffer::pipe::PipeBuffer;
use crate::file::open_file;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::process::Process;
use crate::util::lock::Mutex;
use crate::util::ptr::arc::Arc;
use crate::util::TryDefault;
use core::ffi::c_int;
use macros::syscall;

#[syscall]
pub fn pipe(pipefd: SyscallPtr<[c_int; 2]>) -> Result<i32, Errno> {
	let proc_mutex = Process::current_assert();
	let proc = proc_mutex.lock();

	let mem_space = proc.get_mem_space().unwrap();
	let mut mem_space_guard = mem_space.lock();
	let pipefd_slice = pipefd
		.get_mut(&mut mem_space_guard)?
		.ok_or(errno!(EFAULT))?;

	let fds_mutex = proc.get_fds().unwrap();
	let mut fds = fds_mutex.lock();

	// Create pipe
	let loc = buffer::register(None, Arc::new(Mutex::new(PipeBuffer::try_default()?))?)?;
	open_file::OpenFile::new(loc.clone(), open_file::O_RDWR)?;

	let fd0 = fds.create_fd(loc.clone(), 0, true, false)?;
	pipefd_slice[0] = fd0.get_id() as _;

	let fd1 = fds.create_fd(loc, 0, false, true)?;
	pipefd_slice[1] = fd1.get_id() as _;

	Ok(0)
}
