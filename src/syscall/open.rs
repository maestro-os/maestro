//! The open system call allows a process to open a file and get a file descriptor.

use crate::errno::Errno;
use crate::errno;
use crate::file::File;
use crate::file::FileContent;
use crate::file::FileType;
use crate::file::Gid;
use crate::file::Mode;
use crate::file::Uid;
use crate::file::fcache;
use crate::file::open_file::FDTarget;
use crate::file::open_file;
use crate::file::path::Path;
use crate::file;
use crate::process::Process;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::regs::Regs;
use crate::util::FailableClone;
use crate::util::container::string::String;
use crate::util::ptr::SharedPtr;

/// Mask of status flags to be kept by an open file description.
const STATUS_FLAGS_MASK: i32 = !(open_file::O_CLOEXEC
	| open_file::O_CREAT
	| open_file::O_DIRECTORY
	| open_file::O_EXCL
	| open_file::O_NOCTTY
	| open_file::O_NOFOLLOW
	| open_file::O_TRUNC);

// TODO Implement all flags

/// Returns the file at the given path `path`.
/// If the file doesn't exist and the O_CREAT flag is set, the file is created, then the function
/// returns it. If the flag is not set, the function returns an error with the appropriate errno.
/// If the file is to be created, the function uses `mode` to set its permissions and `uid and
/// `gid` to set the user ID and group ID.
fn get_file(path: Path, flags: i32, mode: Mode, uid: Uid, gid: Gid)
	-> Result<SharedPtr<File>, Errno> {
	// Tells whether to follow symbolic links on the last component of the path.
	let follow_links = flags & open_file::O_NOFOLLOW == 0;

	let mutex = fcache::get();
	let mut guard = mutex.lock();
	let files_cache = guard.get_mut().as_mut().unwrap();

	// Getting the path of the parent directory
	let mut parent_path = path.failable_clone()?;
	// The file's basename
	let name = parent_path.pop();

	// The parent directory
	let parent_mutex = files_cache.get_file_from_path(&parent_path, uid, gid, true)?;
	let mut parent_guard = parent_mutex.lock();
	let parent = parent_guard.get_mut();

	let file_result = match &name {
		Some(name) => {
			// The file is not the root directory
			files_cache.get_file_from_parent(parent, name.failable_clone()?, uid, gid,
				follow_links)
		},

		None => {
			// The file is the root directory
			files_cache.get_file_from_path(&path, uid, gid, follow_links)
		}
	};

	match file_result {
		// If the file is found, return it
		Ok(file) => Ok(file),

		Err(e) if e.as_int() == errno::ENOENT && flags & open_file::O_CREAT != 0 => {
			// Creating the file
			let name = name.unwrap_or_else(|| String::new());
			files_cache.create_file(parent, name, uid, gid, mode, FileContent::Regular)
		},

		Err(e) => Err(e),
	}
}

/// Performs the open system call.
pub fn open_(pathname: SyscallString, flags: i32, mode: file::Mode) -> Result<i32, Errno> {
	let mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock();
	let proc = guard.get_mut();

	// Getting the path string
	let path = {
		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();
		Path::from_str(pathname.get(&mem_space_guard)?.ok_or(errno!(EFAULT))?, true)?
	};

	let mode = mode & !proc.get_umask();
	let uid = proc.get_euid();
	let gid = proc.get_egid();

	// Getting the file
	let abs_path = super::util::get_absolute_path(&proc, path)?;
	let file = get_file(abs_path, flags, mode, uid, gid)?;

	// If O_DIRECTORY is set and the file is not a directory, return an error
	if flags & open_file::O_DIRECTORY != 0
		&& file.lock().get().get_file_type() != FileType::Directory {
		return Err(errno!(ENOTDIR));
	}

	// Create and return the file descriptor
	let fd = proc.create_fd(flags & STATUS_FLAGS_MASK, FDTarget::File(file))?;
	Ok(fd.get_id() as _)
}

/// The implementation of the `open` syscall.
pub fn open(regs: &Regs) -> Result<i32, Errno> {
	let pathname: SyscallString = (regs.ebx as usize).into();
	let flags = regs.ecx as i32;
	let mode = regs.edx as file::Mode;

	open_(pathname, flags, mode)
}
