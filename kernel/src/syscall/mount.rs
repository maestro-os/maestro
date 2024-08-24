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
		mem_space::copy::{SyscallPtr, SyscallString},
		Process,
	},
	syscall::Args,
};
use core::ffi::{c_ulong, c_void};
use utils::{
	errno,
	errno::{EResult, Errno},
};

pub fn mount(
	Args((source, target, filesystemtype, mountflags, _data)): Args<(
		SyscallString,
		SyscallString,
		SyscallString,
		c_ulong,
		SyscallPtr<c_void>,
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
	let target_file = vfs::get_file_from_path(&target_path, &rs)?;
	// Check the target is a directory
	if target_file.get_type()? != FileType::Directory {
		return Err(errno!(ENOTDIR));
	}
	// TODO Use `data`
	// Create mountpoint
	mountpoint::create(
		mount_source,
		Some(fs_type),
		mountflags,
		target_path,
		target_file.node.location.clone(),
	)?;
	Ok(0)
}
