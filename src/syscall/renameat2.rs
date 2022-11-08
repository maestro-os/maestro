//! The `renameat2` allows to rename a file.

use crate::errno::Errno;
use crate::file;
use crate::file::vfs;
use crate::file::FileType;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::regs::Regs;
use crate::process::Process;
use crate::types::c_int;

/// Flag: Don't replace new path if it exists. Return an error instead.
const RENAME_NOREPLACE: c_int = 1;
/// Flag: Exchanges old and new paths atomically.
const RENAME_EXCHANGE: c_int = 2;
/// TODO doc
const RENAME_WHITEOUT: c_int = 4;

/// The implementation of the `renameat2` system call.
pub fn renameat2(regs: &Regs) -> Result<i32, Errno> {
	let olddirfd = regs.ebx as c_int;
	let oldpath: SyscallString = (regs.ecx as usize).into();
	let newdirfd = regs.edx as c_int;
	let newpath: SyscallString = (regs.esi as usize).into();
	let _flags = regs.edi as c_int;

	let (uid, gid, old_mutex, new_parent_mutex, new_name) = {
		let proc_mutex = Process::get_current().unwrap();
		let proc_guard = proc_mutex.lock();
		let proc = proc_guard.get();

		let uid = proc.get_euid();
		let gid = proc.get_egid();

		let mem_space = proc.get_mem_space().clone().unwrap();
		let mem_space_guard = mem_space.lock();

		let oldpath = oldpath
			.get(&mem_space_guard)?
			.ok_or_else(|| errno!(EFAULT))?;
		let old = super::util::get_file_at(proc_guard, false, olddirfd, oldpath, 0)?;

		let proc_guard = proc_mutex.lock();
		let newpath = newpath
			.get(&mem_space_guard)?
			.ok_or_else(|| errno!(EFAULT))?;
		let (new_parent, new_name) =
			super::util::get_parent_at_with_name(proc_guard, false, newdirfd, newpath)?;

		(uid, gid, old, new_parent, new_name)
	};

	let old_guard = old_mutex.lock();
	let old = old_guard.get_mut();

	let new_parent_guard = new_parent_mutex.lock();
	let new_parent = new_parent_guard.get_mut();

	// TODO Check permissions if sticky bit is set

	let vfs = vfs::get();
	let vfs = vfs.lock();
	let vfs = vfs.get_mut().as_mut().unwrap();

	if new_parent.get_location().mountpoint_id == old.get_location().mountpoint_id {
		// Old and new are both on the same filesystem

		// TODO On fail, undo

		// Create link at new location
		// The `..` entry is already updated by the file system since having the same
		// directory in several locations is not allowed
		vfs.create_link(old, new_parent, new_name, uid, gid)?;

		if old.get_type() != FileType::Directory {
			vfs.remove_file(old, uid, gid)?;
		}
	} else {
		// Old and new are on different filesystems.

		// TODO On fail, undo

		file::util::copy_file(vfs, old, new_parent, new_name)?;
		file::util::remove_recursive(vfs, old, uid, gid)?;
	}

	Ok(0)
}
