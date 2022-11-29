//! The `fstatfs` system call returns information about a mounted file system.

use crate::errno;
use crate::errno::Errno;
use crate::file::fs::Statfs;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::process::Process;
use core::ffi::c_int;
use macros::syscall;

/// The implementation of the `fstatfs` syscall.
#[syscall]
pub fn fstatfs(fd: c_int, buf: SyscallPtr<Statfs>) -> Result<i32, Errno> {
	if fd < 0 {
		return Err(errno!(EBADF));
	}

	let file_mutex = {
		let mutex = Process::get_current().unwrap();
		let guard = mutex.lock();
		let proc = guard.get_mut();

		let fds_mutex = proc.get_fds().unwrap();
		let fds_guard = fds_mutex.lock();
		let fds = fds_guard.get();

		let fd = fds.get_fd(fd as _).ok_or_else(|| errno!(EBADF))?;

		let open_file_mutex = fd.get_open_file();
		let open_file_guard = open_file_mutex.lock();
		let open_file = open_file_guard.get();

		open_file.get_file()?
	};

	let file_guard = file_mutex.lock();
	let file = file_guard.get();

	let mountpoint_mutex = file.get_location().get_mountpoint().unwrap();
	let mountpoint_guard = mountpoint_mutex.lock();
	let mountpoint = mountpoint_guard.get_mut();

	let io_mutex = mountpoint.get_source().get_io()?;
	let io_guard = io_mutex.lock();
	let io = io_guard.get_mut();

	let fs_mutex = mountpoint.get_filesystem();
	let fs_guard = fs_mutex.lock();
	let fs = fs_guard.get();

	let stat = fs.get_stat(io)?;

	// Writing the statfs structure to userspace
	{
		let mutex = Process::get_current().unwrap();
		let guard = mutex.lock();
		let proc = guard.get_mut();

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();

		let buf = buf
			.get_mut(&mem_space_guard)?
			.ok_or_else(|| errno!(EFAULT))?;
		*buf = stat;
	}

	Ok(0)
}
