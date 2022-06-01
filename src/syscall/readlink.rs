//! The `readlink` syscall allows to read the target of a symbolic link.

use core::cmp::min;
use crate::errno::Errno;
use crate::file::FileContent;
use crate::file::fcache;
use crate::file::path::Path;
use crate::process::Process;
use crate::process::mem_space::ptr::SyscallSlice;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::regs::Regs;
use crate::util::FailableClone;
use crate::util;

/// The implementation of the `readlink` syscall.
pub fn readlink(regs: &Regs) -> Result<i32, Errno> {
	let pathname: SyscallString = (regs.ebx as usize).into();
	let buf: SyscallSlice<u8> = (regs.ecx as usize).into();
	let bufsiz = regs.edx as usize;

	let (path, uid, gid) = {
		// Getting the process
		let mutex = Process::get_current().unwrap();
		let mut guard = mutex.lock();
		let proc = guard.get_mut();

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();

		let path = Path::from_str(pathname.get(&mem_space_guard)?.ok_or(errno!(EFAULT))?, true)?;
		(path, proc.get_euid(), proc.get_egid())
	};

	// Getting link's target
	let target = {
		let mutex = fcache::get();
		let mut guard = mutex.lock();
		let files_cache = guard.get_mut().as_mut().unwrap();

		// Getting file
		let file_mutex = files_cache.get_file_from_path(&path, uid, gid, false)?;
		let mut file_guard = file_mutex.lock();
		let file = file_guard.get_mut();

		match file.get_file_content() {
			FileContent::Link(target) => target.failable_clone()?,
			_ => return Err(errno!(EINVAL)),
		}
	};

	// Copying to userspace buffer
	{
		let mutex = Process::get_current().unwrap();
		let mut guard = mutex.lock();
		let proc = guard.get_mut();

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();

		let buffer = buf.get_mut(&mem_space_guard, bufsiz)?.ok_or(errno!(EFAULT))?;
		util::slice_copy(target.as_bytes(), buffer);
	}

	Ok(min(bufsiz, target.len()) as _)
}
