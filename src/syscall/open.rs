//! The open system call allows a process to open a file and get a file descriptor.

use crate::errno::Errno;
use crate::errno;
use crate::file::File;
use crate::file::FileContent;
use crate::file::FileType;
use crate::file::fcache;
use crate::file::file_descriptor::FDTarget;
use crate::file::file_descriptor;
use crate::file::path::Path;
use crate::file;
use crate::process::Process;
use crate::process::Regs;
use crate::util::FailableClone;
use crate::util::ptr::SharedPtr;

// TODO Implement all flags

/// Returns the absolute path to the file.
fn get_file_absolute_path(process: &Process, path_str: &[u8]) -> Result<Path, Errno> {
	let path = Path::from_str(path_str, true)?;
	if !path.is_absolute() {
		let cwd = process.get_cwd();
		let mut absolute_path = cwd.concat(&path)?;
		absolute_path.reduce()?;
		Ok(absolute_path)
	} else {
		Ok(path)
	}
}

/// Returns the file at the given path `path`.
/// If the file doesn't exist and the O_CREAT flag is set, the file is created, then the function
/// returns it. If the flag is not set, the function returns an error with the appropriate errno.
/// If the file is to be created, the function uses `mode` to set its permissions and `uid and
/// `gid` to set the user ID and group ID.
fn get_file(path: Path, flags: i32, mode: file::Mode, uid: u16, gid: u16)
	-> Result<SharedPtr<File>, Errno> {
	let mutex = fcache::get();
	let mut guard = mutex.lock();
	let files_cache = guard.get_mut();

	if let Ok(file) = files_cache.as_mut().unwrap().get_file_from_path(&path) {
		Ok(file)
	} else if flags & file_descriptor::O_CREAT != 0 {
		// Getting the path of the parent directory
		let mut parent = path.failable_clone()?;
		parent.pop();

		// Creating the file
		let name = path[path.get_elements_count() - 1].failable_clone()?;
		let file = File::new(name, FileContent::Regular, uid, gid, mode)?;
		files_cache.as_mut().unwrap().create_file(&parent, file)
	} else {
		Err(errno::ENOENT)
	}
}

/// Performs the open system call.
pub fn open_(pathname: *const u8, flags: i32, mode: file::Mode) -> Result<i32, Errno> {
	let mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock();
	let proc = guard.get_mut();

	// Getting the path string
	let path_str = super::util::get_str(proc, pathname)?;

	// TODO Use effective IDs instead?
	let mode = mode & !proc.get_umask();
	let uid = proc.get_uid();
	let gid = proc.get_gid();

	// Getting the file
	let mut file = get_file(get_file_absolute_path(&proc, path_str)?, flags, mode, uid, gid)?;
	if flags & file_descriptor::O_NOFOLLOW == 0 {
		let path = file::resolve_links(file)?;
		file = get_file(path, flags, mode, uid, gid)?;
	}

	// If O_DIRECTORY is set and the file is not a directory, return an error
	if flags & file_descriptor::O_DIRECTORY != 0
		&& file.lock().get().get_file_type() != FileType::Directory {
		return Err(errno::ENOTDIR);
	}

	// Create and return the file descriptor
	let fd = proc.create_fd(flags, FDTarget::File(file))?;
	Ok(fd.get_id() as _)
}

/// The implementation of the `open` syscall.
pub fn open(regs: &Regs) -> Result<i32, Errno> {
	let pathname = regs.ebx as *const u8;
	let flags = regs.ecx as i32;
	let mode = regs.edx as file::Mode;

	open_(pathname, flags, mode)
}
