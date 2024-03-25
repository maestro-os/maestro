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

//! The mount system call allows to mount a filesystem on the system.

use crate::{
	file::{
		fs, mountpoint, mountpoint::MountSource, path::PathBuf, vfs, vfs::ResolutionSettings,
		FileType,
	},
	process::{
		mem_space::ptr::{SyscallPtr, SyscallString},
		Process,
	},
};
use core::ffi::{c_ulong, c_void};
use macros::syscall;
use utils::{errno, errno::Errno};

#[syscall]
pub fn mount(
	source: SyscallString,
	target: SyscallString,
	filesystemtype: SyscallString,
	mountflags: c_ulong,
	_data: SyscallPtr<c_void>,
) -> Result<i32, Errno> {
	let (mount_source, target_path, fs_type, rs) = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		// Check permission
		if !proc.access_profile.is_privileged() {
			return Err(errno!(EPERM));
		}

		let rs = ResolutionSettings::for_process(&proc, true);

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();

		// Get strings
		let source_slice = source.get(&mem_space_guard)?.ok_or(errno!(EFAULT))?;
		let target_slice = target.get(&mem_space_guard)?.ok_or(errno!(EFAULT))?;
		let filesystemtype_slice = filesystemtype
			.get(&mem_space_guard)?
			.ok_or(errno!(EFAULT))?;

		// Get the mount source
		let mount_source = MountSource::new(source_slice)?;

		// Get the target directory
		let target_path = PathBuf::try_from(target_slice)?;

		let fs_type = fs::get_type(filesystemtype_slice).ok_or(errno!(ENODEV))?;

		(mount_source, target_path, fs_type, rs)
	};

	let target_file_mutex = vfs::get_file_from_path(&target_path, &rs)?;
	let target_file = target_file_mutex.lock();
	// Check the target is a directory
	if target_file.get_type() != FileType::Directory {
		return Err(errno!(ENOTDIR));
	}
	let target_location = target_file.get_location().clone();

	// TODO Use `data`
	// Create mountpoint
	mountpoint::create(
		mount_source,
		Some(fs_type),
		mountflags,
		target_path,
		target_location,
	)?;

	Ok(0)
}
