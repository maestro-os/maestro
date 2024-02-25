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

use crate::errno;
use crate::errno::Errno;
use crate::file::fs;
use crate::file::mountpoint;
use crate::file::mountpoint::MountSource;
use crate::file::path::Path;
use crate::file::vfs;
use crate::file::FileType;
use crate::process::mem_space::ptr::SyscallPtr;
use crate::process::mem_space::ptr::SyscallString;
use crate::process::Process;
use crate::util::TryClone;
use core::ffi::c_ulong;
use core::ffi::c_void;
use macros::syscall;

#[syscall]
pub fn mount(
	source: SyscallString,
	target: SyscallString,
	filesystemtype: SyscallString,
	mountflags: c_ulong,
	_data: SyscallPtr<c_void>,
) -> Result<i32, Errno> {
	let (mount_source, fs_type, target_path) = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let mem_space = proc.get_mem_space().unwrap();
		let mem_space_guard = mem_space.lock();

		let cwd = proc.chroot.try_clone()?.concat(&proc.cwd)?;

		// Get strings
		let source_slice = source.get(&mem_space_guard)?.ok_or(errno!(EFAULT))?;
		let target_slice = target.get(&mem_space_guard)?.ok_or(errno!(EFAULT))?;
		let filesystemtype_slice = filesystemtype
			.get(&mem_space_guard)?
			.ok_or(errno!(EFAULT))?;

		// Get the mount source
		let mount_source = MountSource::from_str(source_slice, cwd)?;

		// Get the target file
		let target_path = Path::from_str(target_slice, true)?;
		let target_path = super::util::get_absolute_path(&proc, target_path)?;
		let target_mutex = vfs::get_file_from_path(&target_path, &proc.access_profile, true)?;
		let target_file = target_mutex.lock();

		// Check the target is a directory
		if target_file.get_type() != FileType::Directory {
			return Err(errno!(ENOTDIR));
		}

		// TODO Check for loop between source and target

		let fs_type = fs::get_type(filesystemtype_slice).ok_or(errno!(ENODEV))?;

		(mount_source, fs_type, target_path)
	};

	// TODO Use `data`
	// Create mountpoint
	mountpoint::create(mount_source, Some(fs_type), mountflags, target_path)?;

	Ok(0)
}
