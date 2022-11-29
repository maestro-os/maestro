//! The pipe system call allows to create a pipe.

use core::ffi::c_int;
use crate::errno::Errno;
use crate::file::open_file;
use crate::file::vfs;
use crate::process::Process;
use crate::process::mem_space::ptr::SyscallPtr;
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

	// Create pipe
	let loc = {
		let vfs_mutex = vfs::get();
		let vfs_guard = vfs_mutex.lock();
		let vfs = vfs_guard.get_mut().as_mut().unwrap();

		let loc = vfs.alloc_virt_location()?;
		vfs.get_fifo(&loc)?;

		loc
	};

	let fd0 = proc.create_fd(loc.clone(), open_file::O_RDONLY)?;
	let fd1 = proc.create_fd(loc, open_file::O_WRONLY)?;

	pipefd_slice[0] = fd0.get_id() as _;
	pipefd_slice[1] = fd1.get_id() as _;
	Ok(0)
}
