//! The `fstatfs64` system call returns information about a mounted file system.

use crate::errno;
use crate::errno::Errno;
use crate::file::fs::Statfs;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::process::Process;
use core::ffi::c_int;
use macros::syscall;

#[syscall]
pub fn fstatfs64(fd: c_int, _sz: usize, buf: SyscallPtr<Statfs>) -> Result<i32, Errno> {
	// TODO use `sz`

	if fd < 0 {
		return Err(errno!(EBADF));
	}

	let file_mutex = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let fds_mutex = proc.get_fds().unwrap();
		let fds = fds_mutex.lock();

		let fd = fds.get_fd(fd as _).ok_or_else(|| errno!(EBADF))?;

		let open_file_mutex = fd.get_open_file()?;
		let open_file = open_file_mutex.lock();

		open_file.get_file()?
	};

	let file = file_mutex.lock();

	let mountpoint_mutex = file.get_location().get_mountpoint().unwrap();
	let mountpoint = mountpoint_mutex.lock();

	let io_mutex = mountpoint.get_source().get_io()?;
	let mut io = io_mutex.lock();

	let fs_mutex = mountpoint.get_filesystem();
	let fs = fs_mutex.lock();

	let stat = fs.get_stat(&mut *io)?;

	// Writing the statfs structure to userspace
	{
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let mem_space = proc.get_mem_space().unwrap();
		let mut mem_space_guard = mem_space.lock();

		let buf = buf
			.get_mut(&mut mem_space_guard)?
			.ok_or_else(|| errno!(EFAULT))?;
		*buf = stat;
	}

	Ok(0)
}
