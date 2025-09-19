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

//! Mountpoint system calls.

use crate::{
	file::{
		FileType, fs,
		perm::AccessProfile,
		vfs,
		vfs::{mountpoint, mountpoint::MountSource},
	},
	memory::user::{UserPtr, UserString},
};
use core::ffi::{c_int, c_ulong, c_void};
use utils::{errno, errno::EResult};

pub fn mount(
	source: UserString,
	target: UserString,
	filesystemtype: UserString,
	mountflags: c_ulong,
	_data: UserPtr<c_void>,
) -> EResult<usize> {
	if !AccessProfile::cur_task().is_privileged() {
		return Err(errno!(EPERM));
	}
	// Read arguments
	let source_slice = source.copy_from_user()?.ok_or(errno!(EFAULT))?;
	let mount_source = MountSource::new(&source_slice)?;
	let target = target.copy_path_from_user()?;
	let filesystemtype_slice = filesystemtype.copy_from_user()?.ok_or(errno!(EFAULT))?;
	let fs_type = fs::get_type(&filesystemtype_slice).ok_or(errno!(ENODEV))?;
	// Get target file
	let target = vfs::get_file_from_path(&target, true)?;
	// Check the target is a directory
	if target.get_type()? != FileType::Directory {
		return Err(errno!(ENOTDIR));
	}
	// TODO Use `data`
	// Create mountpoint
	mountpoint::create(mount_source, Some(fs_type), mountflags as _, Some(target))?;
	Ok(0)
}

pub fn umount(target: UserString) -> EResult<usize> {
	umount2(target, 0)
}

pub fn umount2(target: UserString, _flags: c_int) -> EResult<usize> {
	// TODO handle flags
	// Check permission
	if !AccessProfile::cur_task().is_privileged() {
		return Err(errno!(EPERM));
	}
	// Get target directory
	let target = target.copy_path_from_user()?;
	let target = vfs::get_file_from_path(&target, true)?;
	// Remove mountpoint
	mountpoint::remove(target)?;
	Ok(0)
}
