//! The `renameat2` allows to rename a file.

use crate::errno::Errno;
use crate::file::FileContent;
use crate::file::fcache;
use crate::file;
use crate::process::Process;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::regs::Regs;
use crate::types::c_int;
use super::access;

/// The implementation of the `renameat2` system call.
pub fn renameat2(regs: &Regs) -> Result<i32, Errno> {
	let olddirfd = regs.ebx as c_int;
	let oldpath: SyscallString = (regs.ecx as usize).into();
	let newdirfd = regs.edx as c_int;
	let newpath: SyscallString = (regs.esi as usize).into();
	let flags = regs.edi as c_int;

	let follow_links = flags & access::AT_SYMLINK_NOFOLLOW == 0;

	let (uid, gid, old_mutex, new_parent_mutex, new_name) = {
		let proc_mutex = Process::get_current().unwrap();
		let proc_guard = proc_mutex.lock();
		let proc = proc_guard.get();

		let uid = proc.get_euid();
		let gid = proc.get_egid();

		let mem_space = proc.get_mem_space().clone().unwrap();
		let mem_space_guard = mem_space.lock();

		let oldpath = oldpath.get(&mem_space_guard)?.ok_or_else(|| errno!(EFAULT))?;
		let old = super::util::get_file_at(&proc_guard, follow_links, olddirfd, oldpath, flags)?;

		let newpath = newpath.get(&mem_space_guard)?.ok_or_else(|| errno!(EFAULT))?;
		let (new_parent, new_name) = super::util::get_parent_at_with_name(&proc_guard,
			follow_links, newdirfd, newpath)?;

		(uid, gid, old, new_parent, new_name)
	};

	let old_guard = old_mutex.lock();
	let old = old_guard.get_mut();

	let new_parent_guard = new_parent_mutex.lock();
	let new_parent = new_parent_guard.get_mut();

	let fcache_mutex = fcache::get();
	let fcache_guard = fcache_mutex.lock();
	let fcache = fcache_guard.get_mut().as_mut().unwrap();

	if new_parent.get_location().mountpoint_id == old.get_location().mountpoint_id {
		// Old and new are both on the same filesystem
		// TODO On fail, undo
		// TODO Check permissions

		// Create link at new location
		fcache.create_link(old, new_parent, new_name)?;

		// If directory, update the `..` entry
		match old.get_file_content() {
			FileContent::Directory(_entries) => {
				// TODO
			},

			_ => {},
		}

		fcache.remove_file(old, uid, gid)?;
	} else {
		// Old and new are on different filesystems.
		// TODO On fail, undo
		// TODO Check permissions

		file::util::copy_file(fcache, old, new_parent, new_name)?;
		file::util::remove_recursive(fcache, old, uid, gid)?;
	}

	Ok(0)
}
