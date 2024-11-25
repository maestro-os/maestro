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

//! The `statx` system call returns the extended status of a file.

use super::util::at;
use crate::{
	device::DeviceID,
	file::{
		fd::FileDescriptorTable,
		vfs::{mountpoint::MountSource, ResolutionSettings, Resolved},
	},
	process::{
		mem_space::copy::{SyscallPtr, SyscallString},
		Process,
	},
	sync::mutex::Mutex,
	syscall::Args,
};
use core::ffi::{c_int, c_uint};
use utils::{
	collections::path::PathBuf,
	errno,
	errno::{EResult, Errno},
	ptr::arc::Arc,
};

/// A timestamp for the `statx` syscall.
#[repr(C)]
#[derive(Debug)]
struct StatxTimestamp {
	/// Seconds since the Epoch (UNIX time)
	tv_sec: i64,
	/// Nanoseconds since tv_sec
	tv_nsec: u32,
	/// Reserved field.
	__reserved: i32,
}

/// Extended attributes for a file.
#[repr(C)]
#[derive(Debug)]
pub struct Statx {
	/// Mask of bits indicating filled fields
	stx_mask: u32,
	/// Block size for filesystem I/O
	stx_blksize: u32,
	/// Extra file attribute indicators
	stx_attributes: u64,
	/// Number of hard links
	stx_nlink: u32,
	/// User ID of owner
	stx_uid: u32,
	/// Group ID of owner
	stx_gid: u32,
	/// File type and mode
	stx_mode: u16,

	/// Padding.
	__padding0: u16,

	/// Inode number
	stx_ino: u64,
	/// Total size in bytes
	stx_size: u64,
	/// Number of 512B blocks allocated
	stx_blocks: u64,
	/// Mask to show what's supported in stx_attributes
	stx_attributes_mask: u64,

	/// Last access
	stx_atime: StatxTimestamp,
	/// Creation
	stx_btime: StatxTimestamp,
	/// Last status change
	stx_ctime: StatxTimestamp,
	/// Last modification
	stx_mtime: StatxTimestamp,

	/// Major ID (if the file is a device)
	stx_rdev_major: u32,
	/// Minor ID (if the file is a device)
	stx_rdev_minor: u32,
	/// Major ID of the device containing the filesystem where the file resides
	stx_dev_major: u32,
	/// Minor ID of the device containing the filesystem where the file resides
	stx_dev_minor: u32,

	/// Mount ID.
	stx_mnt_id: u64,

	/// Padding.
	__padding1: [u64; 13],
}

pub fn statx(
	Args((dirfd, pathname, flags, _mask, statxbuff)): Args<(
		c_int,
		SyscallString,
		c_int,
		c_uint,
		SyscallPtr<Statx>,
	)>,
	rs: ResolutionSettings,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	// Validation
	if pathname.0.is_none() || statxbuff.0.is_none() {
		return Err(errno!(EINVAL));
	}
	// TODO Implement all flags
	// Get the file
	let pathname = pathname
		.copy_from_user()?
		.map(PathBuf::try_from)
		.transpose()?;
	let Resolved::Found(file) = at::get_file(&fds.lock(), rs, dirfd, pathname.as_deref(), flags)?
	else {
		return Err(errno!(ENOENT));
	};
	// Get file's stat
	let stat = file.stat()?;
	// TODO Use mask?
	// Get the major and minor numbers of the device of the file's filesystem
	let (stx_dev_major, stx_dev_minor) = match file.node().location.get_mountpoint() {
		Some(mp) => match mp.source {
			MountSource::Device(DeviceID {
				major,
				minor,
				..
			}) => (major, minor),
			_ => (0, 0),
		},
		None => (0, 0),
	};
	// Write
	statxbuff.copy_to_user(&Statx {
		stx_mask: !0,      // TODO
		stx_blksize: 512,  // TODO
		stx_attributes: 0, // TODO
		stx_nlink: stat.nlink as _,
		stx_uid: stat.uid as _,
		stx_gid: stat.gid as _,
		stx_mode: stat.mode as _,

		__padding0: 0,

		stx_ino: file.node().location.inode,
		stx_size: stat.size,
		stx_blocks: stat.blocks,
		stx_attributes_mask: 0, // TODO

		stx_atime: StatxTimestamp {
			tv_sec: stat.atime as _,
			tv_nsec: 0, // TODO
			__reserved: 0,
		},
		stx_btime: StatxTimestamp {
			tv_sec: 0,  // TODO
			tv_nsec: 0, // TODO
			__reserved: 0,
		},
		stx_ctime: StatxTimestamp {
			tv_sec: stat.ctime as _,
			tv_nsec: 0, // TODO
			__reserved: 0,
		},
		stx_mtime: StatxTimestamp {
			tv_sec: stat.mtime as _,
			tv_nsec: 0, // TODO
			__reserved: 0,
		},

		stx_rdev_major: stat.dev_major,
		stx_rdev_minor: stat.dev_minor,
		stx_dev_major,
		stx_dev_minor,

		stx_mnt_id: 0, // TODO

		__padding1: [0; 13],
	})?;
	Ok(0)
}
