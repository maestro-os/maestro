//! The mkdir system call allows to create a directory.

use crate::errno::Errno;
use crate::file::FileContent;
use crate::file::fcache;
use crate::file::path::Path;
use crate::file;
use crate::process::Process;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::regs::Regs;
use crate::util::FailableClone;
use crate::util::container::vec::Vec;

/// The implementation of the `mkdir` syscall.
pub fn mkdir(regs: &Regs) -> Result<i32, Errno> {
	let pathname: SyscallString = (regs.ebx as usize).into();
	let mode = regs.ecx as file::Mode;

	let mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock();
	let proc = guard.get_mut();

	let mem_space_guard = proc.get_mem_space().unwrap().lock();

	// The path to the directory to create
	let mut path = Path::from_str(pathname.get(&mem_space_guard)?.ok_or(errno!(EFAULT))?, true)?;
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
