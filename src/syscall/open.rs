//! The open system call allows a process to open a file and get a file
//! descriptor.

use core::ffi::c_int;
use crate::errno::Errno;
use crate::errno;
use crate::file::File;
use crate::file::FileContent;
use crate::file::FileType;
use crate::file::Gid;
use crate::file::Mode;
use crate::file::Uid;
use crate::file::fd::FD_CLOEXEC;
use crate::file::open_file;
use crate::file::path::Path;
use crate::file::vfs;
use crate::file;
use crate::process::Process;
use crate::process::mem_space::ptr::SyscallString;
use crate::util::FailableClone;
use crate::util::ptr::SharedPtr;
use macros::syscall;

/// Mask of status flags to be kept by an open file description.
pub const STATUS_FLAGS_MASK: i32 = !(open_file::O_CLOEXEC
	| open_file::O_CREAT
	| open_file::O_DIRECTORY
	| open_file::O_EXCL
	| open_file::O_NOCTTY
	| open_file::O_NOFOLLOW
	| open_file::O_TRUNC);

// TODO Implement all flags

/// Returns the file at the given path `path`.
/// If the file doesn't exist and the O_CREAT flag is set, the file is created,
/// then the function returns it. If the flag is not set, the function returns
/// an error with the appropriate errno. If the file is to be created, the
/// function uses `mode` to set its permissions and `uid and `gid` to set the
/// user ID and group ID.
fn get_file(
	path: Path,
	flags: i32,
	mode: Mode,
	uid: Uid,
	gid: Gid,
) -> Result<SharedPtr<File>, Errno> {
	// Tells whether to follow symbolic links on the last component of the path.
	let follow_links = flags & open_file::O_NOFOLLOW == 0;

	let mutex = vfs::get();
	let guard = mutex.lock();
	let vfs = guard.get_mut().as_mut().unwrap();

	if flags & open_file::O_CREAT != 0 {
		// Getting the path of the parent directory
		let mut parent_path = path.failable_clone()?;
		// The file's basename
		let name = parent_path.pop().ok_or_else(|| errno!(ENOENT))?;

		// The parent directory
		let parent_mutex = vfs.get_file_from_path(&parent_path, uid, gid, true)?;
		let parent_guard = parent_mutex.lock();
		let parent = parent_guard.get_mut();

		let file_result =
			vfs.get_file_from_parent(parent, name.failable_clone()?, uid, gid, follow_links);
		match file_result {
			// If the file is found, return it
			Ok(file) => Ok(file),

			Err(e) if e.as_int() == errno::ENOENT => {
				// Creating the file
				vfs.create_file(parent, name, uid, gid, mode, FileContent::Regular)
			}

			Err(e) => Err(e),
		}
	} else {
		// The file is the root directory
		vfs.get_file_from_path(&path, uid, gid, follow_links)
	}
}

/// The function handles open flags.
///
/// Arguments:
/// - `file` is the file.
/// - `flags` is the set of flags provided by userspace.
/// - `uid` is the UID of the process openning the file.
/// - `gid` is the GID of the process openning the file.
///
/// The following informations are returned:
/// - Whether the file is open for reading
/// - Whether the file is open for writing
/// - Whether the file descriptor is open with close-on-exec.
pub fn handle_flags(
	file: &mut File,
	flags: i32,
	uid: Uid,
	gid: Gid
) -> Result<(bool, bool, bool), Errno> {
	let (read, write) = match flags & 0b11 {
		open_file::O_RDONLY => (true, false),
		open_file::O_WRONLY => (false, true),
		open_file::O_RDWR => (true, true),

		_ => return Err(errno!(EINVAL)),
	};
	if read && !file.can_read(uid, gid) {
		return Err(errno!(EACCES));
	}
	if write && !file.can_write(uid, gid) {
		return Err(errno!(EACCES));
	}

	// If O_DIRECTORY is set and the file is not a directory, return an error
	if flags & open_file::O_DIRECTORY != 0 && file.get_type() != FileType::Directory {
		return Err(errno!(ENOTDIR));
	}

	// Truncate the file if necessary
	if flags & open_file::O_TRUNC != 0 {
		file.set_size(0);
	}

	let cloexec = flags & open_file::O_CLOEXEC != 0;

	Ok((read, write, cloexec))
}

/// Performs the open system call.
pub fn open_(pathname: SyscallString, flags: i32, mode: file::Mode) -> Result<i32, Errno> {
	// Getting the path string
	let (path, mode, uid, gid) = {
		let mutex = Process::get_current().unwrap();
		let guard = mutex.lock();
		let proc = guard.get_mut();

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();
		let path = Path::from_str(pathname.get(&mem_space_guard)?.ok_or(errno!(EFAULT))?, true)?;
		let abs_path = super::util::get_absolute_path(&proc, path)?;

		let mode = mode & !proc.get_umask();
		let uid = proc.get_euid();
		let gid = proc.get_egid();

		(abs_path, mode, uid, gid)
	};

	// Getting the file
	let file = get_file(path, flags, mode, uid, gid)?;

	let (loc, read, write, cloexec) = {
		let guard = file.lock();
		let f = guard.get_mut();

		let loc = f.get_location().clone();
		let (read, write, cloexec) = handle_flags(f, flags, uid, gid)?;

		(loc, read, write, cloexec)
	};

	open_file::OpenFile::new(loc.clone(), flags)?;

	// Create the file descriptor
	let mutex = Process::get_current().unwrap();
	let guard = mutex.lock();
	let proc = guard.get_mut();

	let fds_mutex = proc.get_fds().unwrap();
	let fds_guard = fds_mutex.lock();
	let fds = fds_guard.get_mut();

	let mut fd_flags = 0;
	if cloexec {
		fd_flags |= FD_CLOEXEC;
	}

	let fd = fds.create_fd(loc, fd_flags, read, write)?;
	let fd_id = fd.get_id();

	// Flushing file
	match file.lock().get_mut().sync() {
		Err(e) => {
			fds.close_fd(fd_id)?;
			return Err(e);
		}

		_ => {}
	}

	Ok(fd.get_id() as _)
}

/// The implementation of the `open` syscall.
#[syscall]
pub fn open(pathname: SyscallString, flags: c_int, mode: file::Mode) -> Result<i32, Errno> {
	open_(pathname, flags, mode)
}
