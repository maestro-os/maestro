/// This module implements utility functions for system calls.

use core::mem::size_of;
use crate::errno::Errno;
use crate::errno;
use crate::file::File;
use crate::file::FileContent;
use crate::file::Mode;
use crate::file::fcache;
use crate::file::open_file::FDTarget;
use crate::file::path::Path;
use crate::process::Process;
use crate::process::mem_space::ptr::SyscallString;
use crate::util::FailableClone;
use crate::util::container::string::String;
use crate::util::container::vec::Vec;
use crate::util::lock::MutexGuard;
use crate::util::ptr::SharedPtr;

/// Returns the absolute path according to the process's current working directory.
/// `process` is the process.
/// `path` is the path.
pub fn get_absolute_path(process: &Process, path: Path) -> Result<Path, Errno> {
	if !path.is_absolute() {
		let cwd = process.get_cwd();
		cwd.concat(&path)
	} else {
		Ok(path)
	}
}

// TODO Find a safer and cleaner solution
/// Checks that the given array of strings at pointer `ptr` is accessible to process `proc`, then
/// returns its content.
/// If the array or its content strings are not accessible by the process, the function returns an
/// error.
pub unsafe fn get_str_array(process: &Process, ptr: *const *const u8)
	-> Result<Vec<String>, Errno> {
	let mem_space = process.get_mem_space().unwrap();
	let mem_space_guard = mem_space.lock();

	// Checking every elements of the array and counting the number of elements
	let mut len = 0;
	loop {
		let elem_ptr = ptr.add(len);

		// Checking access on elem_ptr
		if !mem_space_guard.get().can_access(elem_ptr as _, size_of::<*const u8>(), true, false) {
			return Err(errno!(EFAULT));
		}

		// Safe because the access is checked before
		let elem = *elem_ptr;
		if elem.is_null() {
			break;
		}

		len += 1;
	}

	// Filling the array
	let mut arr = Vec::with_capacity(len)?;
	for i in 0..len {
		let elem = *ptr.add(i);
		let s: SyscallString = (elem as usize).into();

		arr.push(String::from(s.get(&mem_space_guard)?.unwrap())?)?;
	}

	Ok(arr)
}

/// TODO doc
fn build_path_from_fd(process_guard: &MutexGuard<Process, false>, dirfd: i32, pathname: &[u8])
	-> Result<Path, Errno> {
	let process = process_guard.get();
	let path = Path::from_str(pathname, true)?;

	if path.is_absolute() {
		// Using the given absolute path
		Ok(path)
	} else if dirfd == super::access::AT_FDCWD {
		let cwd = process.get_cwd().failable_clone()?;

		// Using path relative to the current working directory
		cwd.concat(&path)
	} else {
		// Using path relative to the directory given by `dirfd`

		if dirfd < 0 {
			return Err(errno!(EBADF));
		}

		let open_file_mutex = process.get_fd(dirfd as _)
			.ok_or(errno!(EBADF))?
			.get_open_file();
		let open_file_guard = open_file_mutex.lock();
		let open_file = open_file_guard.get();

		match open_file.get_target() {
			FDTarget::File(file_mutex) => {
				let file_guard = file_mutex.lock();
				let file = file_guard.get();

				file.get_path()?.concat(&path)
			},

			_ => Err(errno!(ENOTDIR)),
		}
	}
}

/// Returns the file for the given path `pathname`.
/// `process_guard` is the mutex guard of the current process.
/// `follow_links` tells whether symbolic links may be followed.
/// `dirfd` is the file descriptor of the parent directory.
/// `pathname` is the path relative to the parent directory.
/// `flags` is an integer containing AT_* flags.
pub fn get_file_at(process_guard: &MutexGuard<Process, false>, follow_links: bool, dirfd: i32,
	pathname: &[u8], flags: i32) -> Result<SharedPtr<File>, Errno> {
	let process = process_guard.get();

	if pathname.is_empty() {
		if flags & super::access::AT_EMPTY_PATH != 0 {
			// Using `dirfd` as the file descriptor to the file

			if dirfd < 0 {
				return Err(errno!(EBADF));
			}

			let open_file_mutex = process.get_fd(dirfd as _)
				.ok_or(errno!(EBADF))?
				.get_open_file();
			let open_file_guard = open_file_mutex.lock();
			let open_file = open_file_guard.get();

			open_file.get_target().get_file()
		} else {
			Err(errno!(ENOENT))
		}
	} else {
		let uid = process.get_euid();
		let gid = process.get_egid();

		let path = build_path_from_fd(process_guard, dirfd, pathname)?;

		let fcache = fcache::get();
		let fcache_guard = fcache.lock();
		fcache_guard.get_mut().as_mut().unwrap().get_file_from_path(&path, uid, gid, follow_links)
	}
}

/// TODO doc
pub fn get_parent_at_with_name(process_guard: &MutexGuard<Process, false>, follow_links: bool,
	dirfd: i32, pathname: &[u8]) -> Result<(SharedPtr<File>, String), Errno> {
	if pathname.is_empty() {
		return Err(errno!(ENOENT));
	}

	let mut path = build_path_from_fd(&process_guard, dirfd, pathname)?;
	let name = path.pop().unwrap();

	let fcache_mutex = fcache::get();
	let fcache_guard = fcache_mutex.lock();
	let fcache = fcache_guard.get_mut().as_mut().unwrap();

	let process = process_guard.get();
	let uid = process.get_euid();
	let gid = process.get_egid();

	let parent_mutex = fcache.get_file_from_path(&path, uid, gid, follow_links)?;
	Ok((parent_mutex, name))
}

/// Creates the given file `file` at the given pathname `pathname`.
/// `process_guard` is the mutex guard of the current process.
/// `follow_links` tells whether symbolic links may be followed.
/// `dirfd` is the file descriptor of the parent directory.
/// `pathname` is the path relative to the parent directory.
/// `mode` is the permissions of the newly created file.
/// `content` is the content of the newly created file.
pub fn create_file_at(process_guard: &MutexGuard<Process, false>, follow_links: bool, dirfd: i32,
	pathname: &[u8], mode: Mode, content: FileContent) -> Result<SharedPtr<File>, Errno> {
	let (parent_mutex, name) = get_parent_at_with_name(process_guard, follow_links, dirfd,
		pathname)?;

	let process = process_guard.get();
	let uid = process.get_euid();
	let gid = process.get_egid();
	let umask = process.get_umask();
	let mode = mode & !umask;

	let fcache_mutex = fcache::get();
	let fcache_guard = fcache_mutex.lock();
	let fcache = fcache_guard.get_mut().as_mut().unwrap();

	let parent_guard = parent_mutex.lock();
	let parent = parent_guard.get_mut();

	fcache.create_file(parent, name, uid, gid, mode, content)
}
