//! The open system call allows a process to open a file and get a file
//! descriptor.

use crate::errno;
use crate::errno::EResult;
use crate::errno::Errno;
use crate::file;
use crate::file::fd::FD_CLOEXEC;
use crate::file::open_file;
use crate::file::open_file::OpenFile;
use crate::file::path::Path;
use crate::file::perm::AccessProfile;
use crate::file::vfs;
use crate::file::File;
use crate::file::FileContent;
use crate::file::FileType;
use crate::file::Mode;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::Process;
use crate::util::lock::Mutex;
use crate::util::ptr::arc::Arc;
use crate::util::TryClone;
use core::ffi::c_int;
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
///
/// If the file doesn't exist and the `O_CREAT` flag is set, the file is created,
/// then the function returns it.
/// If the flag is not set, the function returns an error with the appropriate errno.
///
/// If the file is to be created, the function uses `mode` to set its permissions and the provided
/// access profile to set the user ID and group ID.
///
/// The access profile is also used to check permissions.
fn get_file(
	path: Path,
	flags: i32,
	mode: Mode,
	access_profile: &AccessProfile,
) -> EResult<Arc<Mutex<File>>> {
	// Tells whether to follow symbolic links on the last component of the path.
	let follow_links = flags & open_file::O_NOFOLLOW == 0;

	if flags & open_file::O_CREAT != 0 {
		// Get the path of the parent directory
		let mut parent_path = path;
		// The file's basename
		let name = parent_path.pop().ok_or_else(|| errno!(ENOENT))?;

		// The parent directory
		let parent_mutex = vfs::get_file_from_path(&parent_path, access_profile, true)?;
		let mut parent = parent_mutex.lock();

		let file_result = vfs::get_file_from_parent(
			&mut parent,
			name.try_clone()?,
			access_profile,
			follow_links,
		);
		let file = match file_result {
			// If the file is found, return it
			Ok(file) => file,

			// Else, create it
			Err(e) if e.as_int() == errno::ENOENT => vfs::create_file(
				&mut parent,
				name,
				access_profile,
				mode,
				FileContent::Regular,
			)?,

			e => return e,
		};
		// Get file type. There cannot be a race condition since the type of a file cannot be
		// changed
		let file_type = file.lock().get_type();
		match file_type {
			// Cannot open symbolic links themselves
			FileType::Link => Err(errno!(ELOOP)),
			_ => Ok(file),
		}
	} else {
		// The file is the root directory
		vfs::get_file_from_path(&path, access_profile, follow_links)
	}
}

/// The function checks the system call's flags and performs the action associated with some of
/// them.
///
/// Arguments:
/// - `file` is the file
/// - `flags` is the set of flags provided by userspace
/// - `access_profile` is the access profile to check permissions
pub fn handle_flags(file: &mut File, flags: i32, access_profile: &AccessProfile) -> EResult<()> {
	let (read, write) = match flags & 0b11 {
		open_file::O_RDONLY => (true, false),
		open_file::O_WRONLY => (false, true),
		open_file::O_RDWR => (true, true),
		_ => return Err(errno!(EINVAL)),
	};
	if read && !access_profile.can_read_file(file) {
		return Err(errno!(EACCES));
	}
	if write && !access_profile.can_write_file(file) {
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

	Ok(())
}

/// Performs the open system call.
pub fn open_(pathname: SyscallString, flags: i32, mode: file::Mode) -> EResult<i32> {
	let proc_mutex = Process::current_assert();
	let (path, mode, ap, fds_mutex) = {
		let proc = proc_mutex.lock();

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();
		let path = Path::from_str(pathname.get(&mem_space_guard)?.ok_or(errno!(EFAULT))?, true)?;
		let abs_path = super::util::get_absolute_path(&proc, path)?;

		let mode = mode & !proc.umask;

		let fds_mutex = proc.get_fds().unwrap();
		(abs_path, mode, proc.access_profile, fds_mutex)
	};

	// Get file
	let file_mutex = get_file(path, flags, mode, &ap)?;
	let mut file = file_mutex.lock();

	// Handle flags
	handle_flags(&mut file, flags, &ap)?;
	drop(file);

	// Create open file description
	let open_file = OpenFile::new(file_mutex.clone(), flags);

	// Create FD
	let mut fd_flags = 0;
	if flags & open_file::O_CLOEXEC != 0 {
		fd_flags |= FD_CLOEXEC;
	}
	let mut fds = fds_mutex.lock();
	let fd = fds.create_fd(fd_flags, open_file)?;
	let fd_id = fd.get_id();

	// TODO remove?
	// Flush file
	let file = file_mutex.lock();
	if let Err(e) = file.sync() {
		fds.close_fd(fd_id)?;
		return Err(e);
	}

	Ok(fd_id as _)
}

#[syscall]
pub fn open(pathname: SyscallString, flags: c_int, mode: file::Mode) -> Result<i32, Errno> {
	open_(pathname, flags, mode)
}
