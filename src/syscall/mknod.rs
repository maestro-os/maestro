//! The `mknod` system call allows to create a new node on a filesystem.

use crate::device::id;
use crate::errno::Errno;
use crate::errno;
use crate::file::File;
use crate::file::FileContent;
use crate::file::FileType;
use crate::file::fcache;
use crate::file::path::Path;
use crate::file;
use crate::process::Process;
use crate::process::Regs;
use crate::util::FailableClone;

/// The implementation of the `getuid` syscall.
pub fn mknod(regs: &Regs) -> Result<i32, Errno> {
	let pathname = regs.ebx as *const u8;
	let mode = regs.ecx as file::Mode;
	let dev = regs.edx as u64;

	let (path, umask, uid, gid) = {
		// Getting the process
		let mutex = Process::get_current().unwrap();
		let mut guard = mutex.lock();
		let proc = guard.get_mut();

		let umask = proc.get_umask();
		let uid = proc.get_uid();
		let gid = proc.get_gid();
		(Path::from_str(super::util::get_str(proc, pathname)?, true)?, umask, uid, gid)
	};

	if path.is_empty() {
		return Err(errno::EEXIST);
	}

	let mode = mode & !umask;
	let file_type = FileType::from_mode(mode).ok_or(errno::EPERM)?;

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
		FileType::Fifo => FileContent::Fifo(0), // TODO Get an ID
		FileType::Socket => FileContent::Socket(0), // TODO Get an ID
		FileType::BlockDevice => FileContent::BlockDevice {
			major,
			minor,
		},
		FileType::CharDevice => FileContent::CharDevice {
			major,
			minor,
		},

		_ => return Err(errno::EPERM),
	};

	// Creating the node
	let file = File::new(name, file_content, uid, gid, mode)?;
	{
		let mutex = fcache::get();
		let mut guard = mutex.lock();
		let files_cache = guard.get_mut();

		files_cache.as_mut().unwrap().create_file(&parent_path, file)?;
	}

	Ok(0)
}
