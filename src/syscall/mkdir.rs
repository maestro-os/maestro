//! The mkdir system call allows to create a directory.

use crate::errno::Errno;
use crate::file::FileContent;
use crate::file::path::Path;
use crate::file::vfs;
use crate::file;
use crate::process::Process;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::regs::Regs;
use crate::util::FailableClone;
use crate::util::container::hashmap::HashMap;

/// The implementation of the `mkdir` syscall.
pub fn mkdir(regs: &Regs) -> Result<i32, Errno> {
	let pathname: SyscallString = (regs.ebx as usize).into();
	let mode = regs.ecx as file::Mode;

	let (path, mode, uid, gid) = {
		let mutex = Process::get_current().unwrap();
		let guard = mutex.lock();
		let proc = guard.get_mut();

		let mode = mode & !proc.get_umask();
		let uid = proc.get_uid();
		let gid = proc.get_gid();

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();

		// The path to the directory to create
		let mut path =
			Path::from_str(pathname.get(&mem_space_guard)?.ok_or(errno!(EFAULT))?, true)?;
		path = super::util::get_absolute_path(proc, path)?;

		(path, mode, uid, gid)
	};

	// Getting the path of the parent directory
	let mut parent_path = path.failable_clone()?;

	// If the path is not empty, create
	if let Some(name) = parent_path.pop() {
		// Creating the directory
		{
			let mutex = vfs::get();
			let guard = mutex.lock();
			let vfs = guard.get_mut().as_mut().unwrap();

			// Getting parent directory
			let parent_mutex = vfs.get_file_from_path(&parent_path, uid, gid, true)?;
			let parent_guard = parent_mutex.lock();
			let parent = parent_guard.get_mut();

			vfs.create_file(
				parent,
				name,
				uid,
				gid,
				mode,
				FileContent::Directory(HashMap::new()),
			)?;
		}
	}

	Ok(0)
}
