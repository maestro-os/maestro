//! The `chmod` system call allows change the permissions on a file.

use crate::errno::Errno;
use crate::file::fcache;
use crate::file::path::Path;
use crate::file;
use crate::process::Process;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::regs::Regs;

/// The implementation of the `chmod` syscall.
pub fn chmod(regs: &Regs) -> Result<i32, Errno> {
	let pathname: SyscallString = (regs.ebx as usize).into();
	let mode = regs.ecx as file::Mode;

	let (path, uid, gid) = {
		let mutex = Process::get_current().unwrap();
		let guard = mutex.lock();
		let proc = guard.get_mut();

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();

		let path = pathname.get(&mem_space_guard)?.ok_or_else(|| errno!(EFAULT))?;
		(Path::from_str(path, true)?, proc.get_euid(), proc.get_egid())
	};

	let file_mutex = {
		let mutex = fcache::get();
		let guard = mutex.lock();
		let files_cache = guard.get_mut().as_mut().unwrap();

		files_cache.get_file_from_path(&path, uid, gid, true)?
	};
	let file_guard = file_mutex.lock();
	let file = file_guard.get_mut();

	// Checking permissions
	if uid != file::ROOT_UID && uid != file.get_uid() {
		return Err(errno!(EPERM));
	}

	file.set_permissions(mode);
	file.sync()?;

	Ok(0)
}
