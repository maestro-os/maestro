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

//! The statx system call returns the extended status of a file.

use super::util::at;
use crate::{
	file::{
		mountpoint::MountSource,
		path::PathBuf,
		vfs::{ResolutionSettings, Resolved},
	},
	process::{
		mem_space::ptr::{SyscallPtr, SyscallString},
		Process,
	},
};
use core::ffi::{c_int, c_uint};
use macros::syscall;
use utils::{errno, errno::Errno, io::IO};

/// Structure representing a timestamp with the statx syscall.
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

/// Structure containing the extended attributes for a file.
#[repr(C)]
#[derive(Debug)]
struct Statx {
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

#[syscall]
pub fn statx(
	dirfd: c_int,
	pathname: SyscallString,
	flags: c_int,
	_mask: c_uint,
	statxbuff: SyscallPtr<Statx>,
) -> Result<i32, Errno> {
	// Validation
	if pathname.is_null() || statxbuff.is_null() {
		return Err(errno!(EINVAL));
	}
	// TODO Implement all flags
	// Get the file
	let (fds_mutex, path, rs) = {
		let proc_mutex = Process::current_assert();
		let proc = proc_mutex.lock();

		let rs = ResolutionSettings::for_process(&proc, true);

		let mem_space = proc.get_mem_space().unwrap().clone();
		let mem_space_guard = mem_space.lock();

		let fds_mutex = proc.file_descriptors.clone().unwrap();

		let path = pathname
			.get(&mem_space_guard)?
			.ok_or_else(|| errno!(EFAULT))?;
		let path = PathBuf::try_from(path)?;

		(fds_mutex, path, rs)
	};
	let fds = fds_mutex.lock();
	let Resolved::Found(file_mutex) = at::get_file(&fds, rs, dirfd, &path, flags)? else {
		return Err(errno!(ENOENT));
	};
	let file = file_mutex.lock();
	// TODO Use mask?
	// Get the major and minor numbers of the device of the file's filesystem
	let (stx_dev_major, stx_dev_minor) = {
		if let Some(mountpoint_mutex) = file.get_location().get_mountpoint() {
			// TODO Clean: This is a quick fix to avoid a deadlock because vfs is also using
			// the mountpoint and locking vfs requires disabling interrupts
			crate::idt::wrap_disable_interrupts(|| {
				let mountpoint = mountpoint_mutex.lock();
				match mountpoint.get_source() {
					MountSource::Device {
						major,
						minor,
						..
					} => (*major, *minor),
					_ => (0, 0),
				}
			})
		} else {
			(0, 0)
		}
	};
	let inode = file.get_location().get_inode();
	// Fill the structure
	let statx_val = Statx {
		stx_mask: !0,      // TODO
		stx_blksize: 512,  // TODO
		stx_attributes: 0, // TODO
		stx_nlink: file.get_hard_links_count() as _,
		stx_uid: file.get_uid() as _,
		stx_gid: file.get_gid() as _,
		stx_mode: file.get_mode() as _,

		__padding0: 0,

		stx_ino: inode,
		stx_size: file.get_size(),
		stx_blocks: file.blocks_count,
		stx_attributes_mask: 0, // TODO

		stx_atime: StatxTimestamp {
			tv_sec: file.atime as _,
			tv_nsec: 0, // TODO
			__reserved: 0,
		},
		stx_btime: StatxTimestamp {
			tv_sec: 0,  // TODO
			tv_nsec: 0, // TODO
			__reserved: 0,
		},
		stx_ctime: StatxTimestamp {
			tv_sec: file.ctime as _,
			tv_nsec: 0, // TODO
			__reserved: 0,
		},
		stx_mtime: StatxTimestamp {
			tv_sec: file.mtime as _,
			tv_nsec: 0, // TODO
			__reserved: 0,
		},

		stx_rdev_major: file.dev_major,
		stx_rdev_minor: file.dev_minor,
		stx_dev_major,
		stx_dev_minor,

		stx_mnt_id: 0, // TODO

		__padding1: [0; 13],
	};
	// Write structure
	let proc_mutex = Process::current_assert();
	let proc = proc_mutex.lock();
	let mem_space = proc.get_mem_space().unwrap();
	let mut mem_space_guard = mem_space.lock();
	let statx = statxbuff
		.get_mut(&mut mem_space_guard)?
		.ok_or(errno!(EFAULT))?;
	*statx = statx_val;
	Ok(0)
}
