//! The `readlink` syscall allows to read the target of a symbolic link.

use crate::errno::Errno;
use crate::file::path::Path;
use crate::file::vfs;
use crate::file::FileContent;
use crate::process::mem_space::ptr::SyscallSlice;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::Process;
use crate::util;
use core::cmp::min;
use macros::syscall;

#[syscall]
pub fn readlink(
	pathname: SyscallString,
	buf: SyscallSlice<u8>,
	bufsiz: usize,
) -> Result<i32, Errno> {
	let proc_mutex = Process::current_assert();
	let proc = proc_mutex.lock();

	let mem_space_mutex = proc.get_mem_space().unwrap();
	let mut mem_space = mem_space_mutex.lock();

	// Get file's path
	let path = pathname.get(&mem_space)?.ok_or(errno!(EFAULT))?;
	let path = Path::from_str(path, true)?;
	let path = super::util::get_absolute_path(&proc, path)?;

	// Get link's target
	let file_mutex = {
		let vfs_mutex = vfs::get();
		let mut vfs = vfs_mutex.lock();
		let vfs = vfs.as_mut().unwrap();

		vfs.get_file_from_path(&path, proc.euid, proc.egid, false)
	}?;
	let file = file_mutex.lock();
	let target = match file.get_content() {
		FileContent::Link(target) => target,
		_ => return Err(errno!(EINVAL)),
	};

	// Copy to userspace buffer
	let buffer = buf.get_mut(&mut mem_space, bufsiz)?.ok_or(errno!(EFAULT))?;
	util::slice_copy(target.as_bytes(), buffer);

	Ok(min(bufsiz, target.len()) as _)
}
