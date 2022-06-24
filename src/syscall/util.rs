/// This module implements utility functions for system calls.

use core::mem::size_of;
use crate::errno::Errno;
use crate::errno;
use crate::file::File;
use crate::file::fcache;
use crate::file::open_file::FDTarget;
use crate::file::path::Path;
use crate::process::Process;
use crate::process::mem_space::ptr::SyscallString;
use crate::util::FailableClone;
use crate::util::container::string::String;
use crate::util::container::vec::Vec;
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

/// Returns the file for the given path `pathname`.
/// `process` is the current process.
/// `follow_links` tells whether symbolic links may be followed.
/// `dirfd` is the file descriptor of the parent directory.
/// `pathname` is the path relative to the parent directory.
/// `flags` is an integer containing AT_* flags.
/// The other arguments are the one given by the system call.
pub fn get_file_at(process: &Process, follow_links: bool, dirfd: i32, pathname: SyscallString,
	flags: i32) -> Result<SharedPtr<File>, Errno> {
	let mem_space = process.get_mem_space().unwrap();
	let mem_space_guard = mem_space.lock();
	let pathname = pathname.get(&mem_space_guard)?.ok_or_else(|| errno!(EFAULT))?;

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

			match open_file.get_target() {
				FDTarget::File(f) => Ok(f.clone()),
				_ => Err(errno!(EBADF)), // TODO Check if correct
			}
		} else {
			Err(errno!(ENOENT))
		}
	} else {
		let path = Path::from_str(pathname, true)?;
		let final_path = {
			if path.is_absolute() {
				// Using the given absolute path
				path
			} else if dirfd == super::access::AT_FDCWD {
				let cwd = process.get_cwd().failable_clone()?;

				// Using path relative to the current working directory
				cwd.concat(&path)?
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

						file.get_path()?.concat(&path)?
					},

					_ => return Err(errno!(ENOTDIR)),
				}
			}
		};

		let fcache = fcache::get();
		let fcache_guard = fcache.lock();
		fcache_guard.get_mut().as_mut().unwrap().get_file_from_path(&final_path,
			process.get_euid(), process.get_gid(), follow_links)
	}
}
