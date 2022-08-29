//! The `fstatfs` system call returns information about a mounted file system.

use core::ffi::c_int;
use crate::errno::Errno;
use crate::errno;
use crate::file::fs::Statfs;
use crate::file::open_file::FDTarget;
use crate::process::Process;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::process::regs::Regs;

/// The implementation of the `fstatfs` syscall.
pub fn fstatfs(regs: &Regs) -> Result<i32, Errno> {
	let fd = regs.ebx as c_int;
	let buf: SyscallPtr<Statfs> = (regs.ecx as usize).into();

	if fd < 0 {
		return Err(errno!(EBADF));
	}

	let file_mutex = {
		let mutex = Process::get_current().unwrap();
		let guard = mutex.lock();
		let proc = guard.get_mut();

		let fd = proc.get_fd(fd as _).ok_or_else(|| errno!(EBADF))?;
		let open_file_mutex = fd.get_open_file();
		let open_file_guard = open_file_mutex.lock();
		let open_file = open_file_guard.get();

		match open_file.get_target() {
			FDTarget::File(file) => file.clone(),
			_ => return Err(errno!(ENOSYS)),
		}
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
