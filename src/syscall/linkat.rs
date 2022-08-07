//! This `linkat` syscall creates a new hard link to a file.

use super::access;
use crate::errno::Errno;
use crate::file::fcache;
use crate::file::FileType;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::regs::Regs;
use crate::process::Process;
use crate::types::c_int;

/// The implementation of the `linkat` system call.
pub fn linkat(regs: &Regs) -> Result<i32, Errno> {
	let olddirfd = regs.ebx as c_int;
	let oldpath: SyscallString = (regs.ecx as usize).into();
	let newdirfd = regs.edx as c_int;
	let newpath: SyscallString = (regs.esi as usize).into();
	let flags = regs.edi as c_int;

	let follow_links = flags & access::AT_SYMLINK_NOFOLLOW == 0;

	let (old_mutex, new_parent_mutex, new_name) = {
		let proc_mutex = Process::get_current().unwrap();
		let proc_guard = proc_mutex.lock();
		let proc = proc_guard.get();

		let mem_space = proc.get_mem_space().clone().unwrap();
		let mem_space_guard = mem_space.lock();

		let oldpath = oldpath
			.get(&mem_space_guard)?
			.ok_or_else(|| errno!(EFAULT))?;
		let old = super::util::get_file_at(proc_guard, follow_links, olddirfd, oldpath, flags)?;

		let proc_guard = proc_mutex.lock();
		let newpath = newpath
			.get(&mem_space_guard)?
			.ok_or_else(|| errno!(EFAULT))?;
		let (new_parent, new_name) =
			super::util::get_parent_at_with_name(proc_guard, follow_links, newdirfd, newpath)?;

		(old, new_parent, new_name)
	};

	let old_guard = old_mutex.lock();
	let old = old_guard.get_mut();

	if old.get_file_type() == FileType::Directory {
		return Err(errno!(EISDIR));
	}

	let new_parent_guard = new_parent_mutex.lock();
	let new_parent = new_parent_guard.get_mut();

	let fcache_mutex = fcache::get();
	let fcache_guard = fcache_mutex.lock();
	let fcache = fcache_guard.get_mut().as_mut().unwrap();

	fcache.create_link(old, new_parent, new_name)?;
	Ok(0)
}
