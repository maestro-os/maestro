//! The open system call allows a process to open a file and get a file descriptor.

use crate::errno::Errno;
use crate::errno;
use crate::file::File;
use crate::file::FileContent;
use crate::file::FileType;
use crate::file::file_descriptor::FDTarget;
use crate::file::file_descriptor;
use crate::file::path::Path;
use crate::file;
use crate::limits;
use crate::process::Process;
use crate::util::FailableClone;
use crate::util::ptr::SharedPtr;
use crate::util;

// TODO Implement all flags

/// Returns the absolute path to the file.
fn get_file_absolute_path(process: &Process, path_str: &str) -> Result<Path, Errno> {
	let path = Path::from_string(path_str, true)?;
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
fn get_file(path: Path, flags: i32, mode: u16, uid: u16, gid: u16)
	-> Result<SharedPtr<File>, Errno> {
	let mutex = file::get_files_cache();
	let mut guard = mutex.lock(true);
	let files_cache = guard.get_mut();

	if let Ok(file) = files_cache.get_file_from_path(&path) {
		Ok(file)
	} else if flags & file_descriptor::O_CREAT != 0 {
		// Getting the path of the parent directory
		let mut parent = path.failable_clone()?;
		parent.pop();

		// Creating the file
		let name = path[path.get_elements_count() - 1].failable_clone()?;
		let file = File::new(name, FileContent::Regular, uid, gid, mode)?;
		files_cache.create_file(&parent, file)
	} else {
		Err(errno::ENOENT)
	}
}

/// Resolves symbolic links and returns the final file. If too many links are to be resolved, the
/// function returns an error.
/// `file` is the starting file. If not a link, the function returns the same file directly.
/// `flags` are the system call's flag.
/// `mode` is used in case the file has to be created and represents its permissions to be set.
/// `uid` is used in case the file has to be created and represents its UID.
/// `gid` is used in case the file has to be created and represents its GID.
fn resolve_links(file: SharedPtr<File>, flags: i32, mode: u16, uid: u16, gid: u16)
	-> Result<SharedPtr<File>, Errno> {
	let mut resolve_count = 0;
	let mut file = file;

	// Resolve links until the current file is not a link
	loop {
		let file_guard = file.lock(true);
		let f = file_guard.get();

		// If the current file is not a link, nothing to resolve
		if f.get_file_type() != FileType::Link {
			break;
		}

		// Get the path of the parent directory of the current file
		let mut parent_path = f.get_path()?;
		parent_path.pop();

		// Resolve the link
		if let FileContent::Link(link_target) = f.get_file_content() {
			let mut path = (parent_path + Path::from_string(link_target.as_str(), false)?)?;
			path.reduce()?;
			drop(file_guard);
			file = get_file(path, flags, mode, uid, gid)?;
		} else {
			unreachable!();
		}

		// If the maximum number of resolutions have been reached, stop
		resolve_count += 1;
		if resolve_count > limits::SYMLOOP_MAX {
			return Err(errno::ELOOP);
		}
	}

	Ok(file)
}

/// The implementation of the `open` syscall.
pub fn open(regs: &util::Regs) -> Result<i32, Errno> {
	let pathname = regs.ebx as *const u8;
	let flags = regs.ecx as i32;
	let mode = regs.edx as u16;

	let mut mutex = Process::get_current().unwrap();
	let mut guard = mutex.lock(false);
	let proc = guard.get_mut();

	// Check the pathname is accessible by the process
	let len = proc.get_mem_space().unwrap().can_access_string(pathname as _, true, false);
	if len.is_none() {
		return Err(errno::EFAULT);
	}
	let len = len.unwrap();
	if len > limits::PATH_MAX {
		return Err(errno::ENAMETOOLONG);
	}

	let path_str = unsafe { // Safe because the address is checked before
		util::ptr_to_str(pathname as _)
	};

	let mode = mode & !proc.get_umask();
	let uid = proc.get_uid();
	let gid = proc.get_gid();

	// Getting the file
	let mut file = get_file(get_file_absolute_path(&proc, path_str)?, flags, mode, uid, gid)?;
	if flags & file_descriptor::O_NOFOLLOW == 0 {
		file = resolve_links(file, flags, mode, uid, gid)?;
	}

	// If O_DIRECTORY is set and the file is not a directory, return an error
	if flags & file_descriptor::O_DIRECTORY != 0 {
		if file.lock(true).get().get_file_type() != FileType::Directory {
			return Err(errno::ENOTDIR);
		}
	}

	// Create and return the file descriptor
	let fd = proc.create_fd(flags, FDTarget::File(file))?;
	Ok(fd.get_id() as _)
}
