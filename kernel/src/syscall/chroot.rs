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

//! The `chroot` system call allows to virtually redefine the system's root for
//! the current process.

use crate::{
	file::{mountpoint, path::Path, vfs::ResolutionSettings, FileType},
	process::{mem_space::ptr::SyscallString, Process},
	vfs,
};
use macros::syscall;
use utils::{errno, errno::Errno};

#[syscall]
pub fn chroot(path: SyscallString) -> Result<i32, Errno> {
	let proc_mutex = Process::current_assert();
	let mut proc = proc_mutex.lock();
	// Check permission
	if !proc.access_profile.is_privileged() {
		return Err(errno!(EPERM));
	}

	let rs = ResolutionSettings {
		root: mountpoint::root_location(),
		..ResolutionSettings::for_process(&proc, true)
	};

	// Get file
	let file = {
		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();

		let path = path.get(&mem_space_guard)?.ok_or(errno!(EFAULT))?;
		let path = Path::new(path)?;

		vfs::get_file_from_path(path, &rs)?
	};
	let file = file.lock();
	if file.get_type() != FileType::Directory {
		return Err(errno!(ENOTDIR));
	}

	proc.chroot = file.get_location().clone();

	Ok(0)
}
