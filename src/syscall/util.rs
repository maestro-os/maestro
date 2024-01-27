/*
 * Copyright 2024 Luc Lenôtre
 *
 * This file is part of Maestro.
 *
 * Maestro is free software: you can redistribute it and/or modify it under the
 * terms of the GNU General Public License as published by the Free Software
 * Foundation, either version 3 of the License, or (at your option) any later
 * version.
 *
 * Maestro is distributed in the hope that it will be useful, but WITHOUT ANY
 * WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR
 * A PARTICULAR PURPOSE. See the GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along with
 * Maestro. If not, see <https://www.gnu.org/licenses/>.
 */

//! This module implements utility functions for system calls.

use crate::errno;
use crate::errno::EResult;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::regs::Regs;
use crate::process::scheduler;
use crate::process::Process;
use crate::process::State;
use crate::util::container::string::String;
use crate::util::container::vec::Vec;
use core::mem::size_of;

// TODO Find a safer and cleaner solution
/// Checks that the given array of strings at pointer `ptr` is accessible to
/// process `proc`, then returns its content.
///
/// If the array or its content strings are not accessible by the process, the
/// function returns an error.
pub unsafe fn get_str_array(process: &Process, ptr: *const *const u8) -> EResult<Vec<String>> {
	let mem_space = process.get_mem_space().unwrap();
	let mem_space_guard = mem_space.lock();

	// Checking every elements of the array and counting the number of elements
	let mut len = 0;
	loop {
		let elem_ptr = ptr.add(len);

		// Checking access on elem_ptr
		if !mem_space_guard.can_access(elem_ptr as _, size_of::<*const u8>(), true, false) {
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
	// TODO collect
	let mut arr = Vec::with_capacity(len)?;
	for i in 0..len {
		let elem = *ptr.add(i);
		let s: SyscallString = (elem as usize).into();

		arr.push(String::try_from(s.get(&mem_space_guard)?.unwrap())?)?;
	}

	Ok(arr)
}

/// Updates the execution flow of the current process according to its state.
///
/// When the state of the current process has been changed, execution may not
/// resume. In which case, the current function handles the execution flow
/// accordingly.
///
/// The function locks the mutex of the current process. Thus, the caller must
/// ensure the mutex isn't already locked to prevent a deadlock.
///
/// If returning, the function returns the mutex lock of the current process.
pub fn handle_proc_state() {
	let proc_mutex = Process::current_assert();
	let proc = proc_mutex.lock();

	match proc.get_state() {
		// The process is executing a signal handler. Make the scheduler jump to it
		State::Running => {
			if proc.is_handling_signal() {
				let regs = proc.regs.clone();
				drop(proc);
				drop(proc_mutex);

				unsafe {
					regs.switch(true);
				}
			}
		}

		// The process is sleeping or has been stopped. Waiting until wakeup
		State::Sleeping | State::Stopped => {
			drop(proc);
			drop(proc_mutex);

			scheduler::end_tick();
		}

		// The process has been killed. Stopping execution and waiting for the next tick
		State::Zombie => {
			drop(proc);
			drop(proc_mutex);

			scheduler::end_tick();
		}
	}
}

/// Checks whether the current syscall must be interrupted to execute a signal.
///
/// If interrupted, the function doesn't return and the control flow jumps
/// directly to handling the signal.
///
/// The function locks the mutex of the current process. Thus, the caller must
/// ensure the mutex isn't already locked to prevent a deadlock.
///
/// `regs` is the registers state passed to the current syscall.
pub fn signal_check(regs: &Regs) {
	let proc_mutex = Process::current_assert();
	let mut proc = proc_mutex.lock();

	if proc.get_next_signal().is_some() {
		// Returning the system call early to resume it later
		let mut r = regs.clone();
		// TODO Clean
		r.eip -= 2; // TODO Handle the case where the instruction isn't two bytes long (sysenter)
		proc.regs = r;
		proc.syscalling = false;

		// Switching to handle the signal
		proc.prepare_switch();

		drop(proc);
		drop(proc_mutex);

		handle_proc_state();
	}
}

/// `*at` system calls allow to perform operations on files without having to redo the whole
/// path-resolution each time.
///
/// This module implements utility functions for those system calls.
pub mod at {
	use crate::errno::EResult;
	use crate::file::fd::FileDescriptorTable;
	use crate::file::path::Path;
	use crate::file::vfs::{ResolutionSettings, Resolved};
	use crate::file::{vfs, FileLocation};
	use core::ffi::c_int;

	/// Special value to be used as file descriptor, telling to take the path relative to the
	/// current working directory.
	pub const AT_FDCWD: c_int = -100;

	/// Flag: If pathname is a symbolic link, do not dereference it: instead return
	/// information about the link itself.
	pub const AT_SYMLINK_NOFOLLOW: c_int = 0x100;
	/// Flag: Perform access checks using the effective user and group IDs.
	pub const AT_EACCESS: c_int = 0x200;
	/// Flag: If pathname is a symbolic link, dereference it.
	pub const AT_SYMLINK_FOLLOW: c_int = 0x400;
	/// Flag: Don't automount the terminal component of `pathname` if it is a directory that is an
	/// automount point.
	pub const AT_NO_AUTOMOUNT: c_int = 0x800;
	/// Flag: If `pathname` is an empty string, operate on the file referred to by `dirfd`.
	pub const AT_EMPTY_PATH: c_int = 0x1000;
	/// Flag: Do whatever `stat` does.
	pub const AT_STATX_SYNC_AS_STAT: c_int = 0x0000;
	/// Flag: Force the attributes to be synchronized with the server.
	pub const AT_STATX_FORCE_SYNC: c_int = 0x2000;
	/// Flag: Don't synchronize anything, but rather take cached information.
	pub const AT_STATX_DONT_SYNC: c_int = 0x4000;

	/// Returns the location of the file pointed to by the given file descriptor.
	///
	/// Arguments:
	/// - `fds` is the file descriptors table
	/// - `fd` is the file descriptor
	///
	/// If the given file descriptor is invalid, the function returns [`errno::EBADF`].
	fn fd_to_loc(fds: &FileDescriptorTable, fd: c_int) -> EResult<FileLocation> {
		if fd < 0 {
			return Err(errno!(EBADF));
		}
		let open_file_mutex = fds
			.get_fd(fd as _)
			.ok_or(errno!(EBADF))?
			.get_open_file()
			.clone();
		let open_file = open_file_mutex.lock();
		Ok(open_file.get_location().clone())
	}

	/// Returns the file for the given path `path`.
	///
	/// Arguments:
	/// - `fds` is the file descriptors table to use
	/// - `rs` is the path resolution settings to use
	/// - `dirfd` is the file descriptor of the parent directory
	/// - `path` is the path relative to the parent directory
	/// - `flags` is the set of `AT_*` flags
	///
	/// **Note**: the `start` field of [`ResolutionSettings`] is used as the current working
	/// directory.
	pub fn get_file<'p>(
		fds: &FileDescriptorTable,
		mut rs: ResolutionSettings,
		dirfd: c_int,
		path: &'p Path,
		flags: c_int,
	) -> EResult<Resolved<'p>> {
		// Prepare resolution settings
		let follow_links = if rs.follow_link {
			flags & AT_SYMLINK_NOFOLLOW == 0
		} else {
			flags & AT_SYMLINK_FOLLOW != 0
		};
		rs.follow_link = follow_links;
		// If not starting from current directory, get location
		if dirfd != AT_FDCWD {
			rs.start = fd_to_loc(fds, dirfd)?;
		}
		if path.is_empty() {
			// Validation
			if flags & AT_EMPTY_PATH == 0 {
				return Err(errno!(ENOENT));
			}
			let file = vfs::get_file_from_location(&rs.start)?;
			Ok(Resolved::Found(file))
		} else {
			vfs::resolve_path(path, &rs)
		}
	}
}
