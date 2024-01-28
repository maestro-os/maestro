//! The `readlink` syscall allows to read the target of a symbolic link.

use crate::errno::Errno;
use crate::file::path::PathBuf;
use crate::file::vfs;
use crate::file::vfs::ResolutionSettings;
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
	// process lock has to be dropped to avoid deadlock with procfs
	let (mem_space_mutex, path, rs) = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let mem_space_mutex = proc.get_mem_space().unwrap().clone();
		let mem_space = mem_space_mutex.lock();

		// Get file's path
		let path = pathname.get(&mem_space)?.ok_or(errno!(EFAULT))?;
		let path = PathBuf::try_from(path)?;

		drop(mem_space);

		let rs = ResolutionSettings::for_process(&proc, false);
		(mem_space_mutex, path, rs)
	};

	// Get link's target
	let file_mutex = vfs::get_file_from_path(&path, &rs)?;
	let file = file_mutex.lock();
	let FileContent::Link(target) = file.get_content() else {
		return Err(errno!(EINVAL));
	};

	// Copy to userspace buffer
	let mut mem_space = mem_space_mutex.lock();
	let buffer = buf.get_mut(&mut mem_space, bufsiz)?.ok_or(errno!(EFAULT))?;
	util::slice_copy(target.as_bytes(), buffer);

	Ok(min(bufsiz, target.len()) as _)
}
