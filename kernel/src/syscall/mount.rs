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
		FileType, fs, vfs,
		vfs::{ResolutionSettings, mountpoint, mountpoint::MountSource},
	},
	memory::user::{UserPtr, UserString},
	syscall::Args,
};
use core::ffi::{c_int, c_ulong, c_void};
use utils::{collections::path::PathBuf, errno, errno::EResult};

pub fn mount(
	Args((source, target, filesystemtype, mountflags, _data)): Args<(
		UserString,
		UserString,
		UserString,
		c_ulong,
		UserPtr<c_void>,
	)>,
	rs: ResolutionSettings,
) -> EResult<usize> {
	if !rs.access_profile.is_privileged() {
		return Err(errno!(EPERM));
	}
	// Read arguments
	let source_slice = source.copy_from_user()?.ok_or(errno!(EFAULT))?;
	let mount_source = MountSource::new(&source_slice)?;
	let target_slice = target.copy_from_user()?.ok_or(errno!(EFAULT))?;
	let target_path = PathBuf::try_from(target_slice)?;
	let filesystemtype_slice = filesystemtype.copy_from_user()?.ok_or(errno!(EFAULT))?;
	let fs_type = fs::get_type(&filesystemtype_slice).ok_or(errno!(ENODEV))?;
	// Get target file
	let target = vfs::get_file_from_path(&target_path, &rs)?;
	// Check the target is a directory
	if target.get_type()? != FileType::Directory {
		return Err(errno!(ENOTDIR));
	}
	// TODO Use `data`
	// Create mountpoint
	mountpoint::create(mount_source, Some(fs_type), mountflags as _, Some(target))?;
	Ok(0)
}

pub fn umount(Args(target): Args<UserString>, rs: ResolutionSettings) -> EResult<usize> {
	umount2(Args((target, 0)), rs)
}

pub fn umount2(
	Args((target, _flags)): Args<(UserString, c_int)>,
	rs: ResolutionSettings,
) -> EResult<usize> {
	// TODO handle flags
	// Check permission
	if !rs.access_profile.is_privileged() {
		return Err(errno!(EPERM));
	}
	// Get target directory
	let target_slice = target.copy_from_user()?.ok_or(errno!(EFAULT))?;
	let target_path = PathBuf::try_from(target_slice)?;
	let target = vfs::get_file_from_path(&target_path, &rs)?;
	// Remove mountpoint
	mountpoint::remove(target)?;
	Ok(0)
}
