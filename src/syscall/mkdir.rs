//! The mkdir system call allows to create a directory.

use crate::errno::Errno;
use crate::errno;
use crate::file::File;
use crate::file::FileContent;
use crate::file::FileType;
use crate::file::path::Path;
use crate::file;
use crate::process::Process;
use crate::util::FailableClone;
use crate::util;

/// The implementation of the `mkdir` syscall.
pub fn mkdir(regs: &util::Regs) -> Result<i32, Errno> {
	let pathname = regs.ebx as *const u8;
	let mode = regs.ebx as u16;

	let mut mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock(false);
	let proc = guard.get_mut();

	// Check the pathname is accessible by the process
	let len = proc.get_mem_space().can_access_string(pathname as _, true, false);
	if len.is_none() {
		return Err(errno::EFAULT);
	}

	let path = Path::from_string(unsafe { // Safe because the address is checked before
		util::ptr_to_str(pathname as _)
	})?;

	if !path.is_empty() {
		let name = path[path.get_elements_count() - 1].failable_clone()?;

		// Getting the path of the parent directory
		let mut parent_path = path.failable_clone()?;
		parent_path.pop();

		let mode = mode & !proc.get_umask();
		let uid = proc.get_uid();
		let gid = proc.get_gid();

		// Creating the directory
		let file = File::new(name, FileType::Directory, FileContent::Other, uid, gid, mode)?;
		{
			let mutex = file::get_files_cache();
			let mut guard = mutex.lock(true);
			let files_cache = guard.get_mut();
			files_cache.create_file(&parent_path, file)?;
		}
	}

	Ok(0)
}
