//! The `mknod` system call allows to create a new node on a filesystem.

use crate::device::id;
use crate::errno::Errno;
use crate::errno;
use crate::file::FileContent;
use crate::file::FileType;
use crate::file::path::Path;
use crate::file::vfs;
use crate::file;
use crate::process::Process;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::regs::Regs;
use crate::util::FailableClone;

/// The implementation of the `getuid` syscall.
pub fn mknod(regs: &Regs) -> Result<i32, Errno> {
	let pathname: SyscallString = (regs.ebx as usize).into();
	let mode = regs.ecx as file::Mode;
	let dev = regs.edx as u64;

	let (path, umask, uid, gid) = {
		// Getting the process
		let mutex = Process::get_current().unwrap();
		let guard = mutex.lock();
		let proc = guard.get_mut();

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();

		let path = Path::from_str(pathname.get(&mem_space_guard)?.ok_or(errno!(EFAULT))?, true)?;
		let path = super::util::get_absolute_path(proc, path)?;

		let umask = proc.get_umask();
		let uid = proc.get_uid();
		let gid = proc.get_gid();
		(path, umask, uid, gid)
	};

	if path.is_empty() {
		return Err(errno!(EEXIST));
	}

	let mode = mode & !umask;
	let file_type = FileType::from_mode(mode).ok_or(errno!(EPERM))?;

	// The file name
	let name = path[path.get_elements_count() - 1].failable_clone()?;

	// Getting the path of the parent directory
	let mut parent_path = path.failable_clone()?;
	parent_path.pop();

	// Getting the major and minor IDs
	let major = id::major(dev);
	let minor = id::minor(dev);

	// The file's content
	let file_content = match file_type {
		FileType::Regular => FileContent::Regular,
		FileType::Fifo => FileContent::Fifo,
		FileType::Socket => FileContent::Socket,
		FileType::BlockDevice => FileContent::BlockDevice { major, minor },
		FileType::CharDevice => FileContent::CharDevice { major, minor },

		_ => return Err(errno!(EPERM)),
	};

	// Creating the node
	{
		let mutex = vfs::get();
		let guard = mutex.lock();
		let vfs = guard.get_mut().as_mut().unwrap();

		// Getting parent directory
		let parent_mutex = vfs.get_file_from_path(&parent_path, uid, gid, true)?;
		let parent_guard = parent_mutex.lock();
		let parent = parent_guard.get_mut();

		vfs.create_file(parent, name, uid, gid, mode, file_content)?;
	}

	Ok(0)
}
