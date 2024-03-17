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

//! The `umount` system call allows to unmount a filesystem previously mounted
//! with `mount`.

use crate::{
	file::{mountpoint, path::Path, vfs, vfs::ResolutionSettings},
	process::{mem_space::ptr::SyscallString, Process},
};
use macros::syscall;
use utils::{errno, errno::Errno};

#[syscall]
pub fn umount(target: SyscallString) -> Result<i32, Errno> {
	let proc_mutex = Process::current_assert();
	let proc = proc_mutex.lock();

	// Check permission
	if !proc.access_profile.is_privileged() {
		return Err(errno!(EPERM));
	}

	let rs = ResolutionSettings::for_process(&proc, true);

	let mem_space = proc.get_mem_space().unwrap();
	let mem_space_guard = mem_space.lock();

	// Get target directory
	let target_slice = target.get(&mem_space_guard)?.ok_or(errno!(EFAULT))?;
	let target_path = Path::new(target_slice)?;
	let target_dir = vfs::get_file_from_path(target_path, &rs)?;
	let target_dir = target_dir.lock();

	// Remove mountpoint
	mountpoint::remove(target_dir.get_location())?;

	Ok(0)
}
