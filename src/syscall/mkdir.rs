//! The mkdir system call allows to create a directory.

use crate::errno::Errno;
use crate::file::FileContent;
use crate::file::fcache;
use crate::file::path::Path;
use crate::file;
use crate::process::Process;
use crate::process::Regs;
use crate::util::FailableClone;
use crate::util::container::vec::Vec;

/// The implementation of the `mkdir` syscall.
pub fn mkdir(regs: &Regs) -> Result<i32, Errno> {
	let pathname = regs.ebx as *const u8;
	let mode = regs.ecx as file::Mode;

	let mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock();
	let proc = guard.get_mut();

	// The path to the directory to create
	let mut path = Path::from_str(super::util::get_str(proc, pathname)?, true)?;
	path = super::util::get_absolute_path(proc, path)?;

	if !path.is_empty() {
		let name = path[path.get_elements_count() - 1].failable_clone()?;

		// Getting the path of the parent directory
		let mut parent_path = path.failable_clone()?;
		parent_path.pop();

		let mode = mode & !proc.get_umask();
		let uid = proc.get_uid();
		let gid = proc.get_gid();

		// Creating the directory
		{
			let mutex = fcache::get();
			let mut guard = mutex.lock();
			let files_cache = guard.get_mut().as_mut().unwrap();

			// Getting parent directory
			let parent_mutex = files_cache.get_file_from_path(&parent_path, uid, gid, true)?;
			let mut parent_guard = parent_mutex.lock();
			let parent = parent_guard.get_mut();

			files_cache.create_file(parent, name, uid, gid, mode,
				FileContent::Directory(Vec::new()))?;
		}
	}

	Ok(0)
}
