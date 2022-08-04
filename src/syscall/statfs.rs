//! The `statfs` system call returns information about a mounted file system.

use crate::errno;
use crate::errno::Errno;
use crate::file::fcache;
use crate::file::fs::Statfs;
use crate::file::path::Path;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::regs::Regs;
use crate::process::Process;

/// The implementation of the `statfs` syscall.
pub fn statfs(regs: &Regs) -> Result<i32, Errno> {
	let path: SyscallString = (regs.ebx as usize).into();
	let buf: SyscallPtr<Statfs> = (regs.ecx as usize).into();

	let (path, uid, gid) = {
		let mutex = Process::get_current().unwrap();
		let guard = mutex.lock();
		let proc = guard.get_mut();

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();

		let path = path.get(&mem_space_guard)?.ok_or_else(|| errno!(EFAULT))?;
		let path = Path::from_str(path, true)?;
		let path = super::util::get_absolute_path(proc, path)?;

		(path, proc.get_euid(), proc.get_egid())
	};

	let file_mutex = {
		let mutex = fcache::get();
		let guard = mutex.lock();
		let files_cache = guard.get_mut().as_mut().unwrap();

		files_cache.get_file_from_path(&path, uid, gid, true)?
	};
	let file_guard = file_mutex.lock();
	let file = file_guard.get_mut();

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
