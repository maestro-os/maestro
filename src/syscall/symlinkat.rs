//! The `symlinkat` syscall allows to create a symbolic link.

use crate::errno::Errno;
use crate::file::FileContent;
use crate::limits;
use crate::process::Process;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::regs::Regs;
use crate::util::container::string::String;
use super::util;

/// The implementation of the `symlinkat` syscall.
pub fn symlinkat(regs: &Regs) -> Result<i32, Errno> {
	let target: SyscallString = (regs.ebx as usize).into();
	let newdirfd = regs.ecx as i32;
	let linkpath: SyscallString = (regs.edx as usize).into();

	let mutex = Process::get_current().unwrap();
	let guard = mutex.lock();
	let proc = guard.get_mut();

	let mem_space = proc.get_mem_space().unwrap();
	let mem_space_guard = mem_space.lock();

	let target_slice = target.get(&mem_space_guard)?.ok_or_else(|| errno!(EFAULT))?;
	if target_slice.len() > limits::SYMLINK_MAX {
		return Err(errno!(ENAMETOOLONG));
	}
	let target = String::from(target_slice)?;

	let linkpath = linkpath.get(&mem_space_guard)?.ok_or_else(|| errno!(EFAULT))?;
	let file_content = FileContent::Link(target);
	util::create_file_at(&guard, true, newdirfd, linkpath, 0, file_content)?;

	Ok(0)
}
