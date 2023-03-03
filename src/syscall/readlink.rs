//! The `readlink` syscall allows to read the target of a symbolic link.

use crate::errno::Errno;
use crate::file::path::Path;
use crate::file::vfs;
use crate::file::FileContent;
use crate::process::mem_space::ptr::SyscallSlice;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::Process;
use crate::util;
use crate::util::FailableClone;
use core::cmp::min;
use macros::syscall;

#[syscall]
pub fn readlink(
	pathname: SyscallString,
	buf: SyscallSlice<u8>,
	bufsiz: usize,
) -> Result<i32, Errno> {
	let (path, uid, gid) = {
		// Getting the process
		let mutex = Process::get_current().unwrap();
		let guard = mutex.lock();
		let proc = guard.get_mut();

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();

		let path = Path::from_str(pathname.get(&mem_space_guard)?.ok_or(errno!(EFAULT))?, true)?;
		let path = super::util::get_absolute_path(proc, path)?;
		(path, proc.get_euid(), proc.get_egid())
	};

	// Getting link's target
	let target = {
		let mutex = vfs::get();
		let guard = mutex.lock();
		let vfs = guard.get_mut().as_mut().unwrap();

		// Getting file
		let file_mutex = vfs.get_file_from_path(&path, uid, gid, false)?;
		let file_guard = file_mutex.lock();
		let file = file_guard.get_mut();

		match file.get_content() {
			FileContent::Link(target) => target.failable_clone()?,
			_ => return Err(errno!(EINVAL)),
		}
	};

	// Copying to userspace buffer
	{
		let mutex = Process::get_current().unwrap();
		let guard = mutex.lock();
		let proc = guard.get_mut();

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();

		let buffer = buf
			.get_mut(&mem_space_guard, bufsiz)?
			.ok_or(errno!(EFAULT))?;
		util::slice_copy(target.as_bytes(), buffer);
	}

	Ok(min(bufsiz, target.len()) as _)
}
