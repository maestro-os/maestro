/// This module implements utility functions for system calls.

use core::mem::size_of;
use crate::errno::Errno;
use crate::errno;
use crate::file::File;
use crate::file::FileContent;
use crate::file::Mode;
use crate::file::open_file::FDTarget;
use crate::file::path::Path;
use crate::file::vfs;
use crate::process::Process;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::regs::Regs;
use crate::process::state::State;
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
pub unsafe fn get_str_array(
	process: &Process,
	ptr: *const *const u8,
) -> Result<Vec<String>, Errno> {
	let mem_space = process.get_mem_space().unwrap();
	let mem_space_guard = mem_space.lock();

	// Checking every elements of the array and counting the number of elements
	let mut len = 0;
	loop {
		let elem_ptr = ptr.add(len);

		// Checking access on elem_ptr
		if !mem_space_guard
			.get()
			.can_access(elem_ptr as _, size_of::<*const u8>(), true, false)
		{
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

/// Builds a path with the given directory file descriptor `dirfd` as a base, concatenated with the
/// given pathname `pathname`.
/// `process_guard` is the guard of the current process.
fn build_path_from_fd(
	process_guard: &MutexGuard<Process, false>,
	dirfd: i32,
	pathname: &[u8],
) -> Result<Path, Errno> {
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

		let open_file_mutex = process
			.get_fd(dirfd as _)
			.ok_or(errno!(EBADF))?
			.get_open_file();
		let open_file_guard = open_file_mutex.lock();
		let open_file = open_file_guard.get();

		match open_file.get_target() {
			FDTarget::File(file_mutex) => {
				let file_guard = file_mutex.lock();
				let file = file_guard.get();

				file.get_path()?.concat(&path)
			}

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
pub fn get_file_at(
	process_guard: MutexGuard<Process, false>,
	follow_links: bool,
	dirfd: i32,
	pathname: &[u8],
	flags: i32,
) -> Result<SharedPtr<File>, Errno> {
	let process = process_guard.get();

	if pathname.is_empty() {
		if flags & super::access::AT_EMPTY_PATH != 0 {
			// Using `dirfd` as the file descriptor to the file

			if dirfd < 0 {
				return Err(errno!(EBADF));
			}

			let open_file_mutex = process
				.get_fd(dirfd as _)
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

		let path = build_path_from_fd(&process_guard, dirfd, pathname)?;

		// Unlocking to avoid deadlock with procfs
		drop(process_guard);

		let vfs = vfs::get();
		let vfs_guard = vfs.lock();
		vfs_guard
			.get_mut()
			.as_mut()
			.unwrap()
			.get_file_from_path(&path, uid, gid, follow_links)
	}
}

/// TODO doc
pub fn get_parent_at_with_name(
	process_guard: MutexGuard<Process, false>,
	follow_links: bool,
	dirfd: i32,
	pathname: &[u8],
) -> Result<(SharedPtr<File>, String), Errno> {
	if pathname.is_empty() {
		return Err(errno!(ENOENT));
	}

	let mut path = build_path_from_fd(&process_guard, dirfd, pathname)?;
	let name = path.pop().unwrap();

	let process = process_guard.get();
	let uid = process.get_euid();
	let gid = process.get_egid();

	// Unlocking to avoid deadlock with procfs
	drop(process_guard);

	let vfs_mutex = vfs::get();
	let vfs_guard = vfs_mutex.lock();
	let vfs = vfs_guard.get_mut().as_mut().unwrap();

	let parent_mutex = vfs.get_file_from_path(&path, uid, gid, follow_links)?;
	Ok((parent_mutex, name))
}

/// Creates the given file `file` at the given pathname `pathname`.
/// `process_guard` is the mutex guard of the current process.
/// `follow_links` tells whether symbolic links may be followed.
/// `dirfd` is the file descriptor of the parent directory.
/// `pathname` is the path relative to the parent directory.
/// `mode` is the permissions of the newly created file.
/// `content` is the content of the newly created file.
pub fn create_file_at(
	process_guard: MutexGuard<Process, false>,
	follow_links: bool,
	dirfd: i32,
	pathname: &[u8],
	mode: Mode,
	content: FileContent,
) -> Result<SharedPtr<File>, Errno> {
	let process = process_guard.get();
	let uid = process.get_euid();
	let gid = process.get_egid();
	let umask = process.get_umask();
	let mode = mode & !umask;

	let (parent_mutex, name) =
		get_parent_at_with_name(process_guard, follow_links, dirfd, pathname)?;

	let vfs_mutex = vfs::get();
	let vfs_guard = vfs_mutex.lock();
	let vfs = vfs_guard.get_mut().as_mut().unwrap();

	let parent_guard = parent_mutex.lock();
	let parent = parent_guard.get_mut();

	vfs.create_file(parent, name, uid, gid, mode, content)
}

/// Updates the execution flow of the current process according to its state.
///
/// When the state of the current process has been changed, execution may not resume. In which
/// case, the current function handles the execcution flow accordingly.
///
/// The functions locks the mutex of the current process. Thus, the caller must ensure the mutex
/// isn't already locked to prevent a deadlock.
///
/// If returning, the function returns the mutex lock of the current process.
pub fn handle_proc_state() {
	let proc_mutex = Process::get_current().unwrap();
	let proc_guard = proc_mutex.lock();
	let proc = proc_guard.get_mut();

	match proc.get_state() {
		// The process is executing a signal handler. Make the scheduler jump to it
		State::Running => {
			if proc.is_handling_signal() {
				let regs = proc.get_regs().clone();
				drop(proc_guard);
				drop(proc_mutex);

				unsafe {
					regs.switch(true);
				}
			}
		}

		// The process is sleeping or has been stopped. Waiting until wakeup
		State::Sleeping(_) | State::Stopped => {
			drop(proc_guard);
			drop(proc_mutex);

			crate::wait();
		}

		// The process has been killed. Stopping execution and waiting for the next tick
		State::Zombie => {
			drop(proc_guard);
			drop(proc_mutex);

			crate::enter_loop();
		}
	}
}

/// Checks whether the current syscall must be interrupted to execute a signal.
///
/// If interrupted, the function doesn't return and the control flow jumps directly to handling the
/// signal.
///
/// The functions locks the mutex of the current process. Thus, the caller must ensure the mutex
/// isn't already locked to prevent a deadlock.
///
/// `regs` is the registers state passed to the current syscall.
pub fn signal_check(regs: &Regs) {
	let proc_mutex = Process::get_current().unwrap();
	let proc_guard = proc_mutex.lock();
	let proc = proc_guard.get_mut();

	if proc.get_next_signal().is_some() {
		// Returning the system call early to resume it later
		let mut r = regs.clone();
		// TODO Clean
		r.eip -= 2; // TODO Handle the case where the instruction insn't two bytes long (sysenter)
		proc.set_regs(r);
		proc.set_syscalling(false);

		// Switching to handle the signal
		proc.prepare_switch();

		drop(proc_guard);
		drop(proc_mutex);

		handle_proc_state();
	}
}
