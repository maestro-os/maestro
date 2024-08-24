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

//! The `fstat64` system call allows get the status of a file.

use crate::{
	device::id::makedev,
	file::{
		fd::FileDescriptorTable,
		perm::{Gid, Uid},
		INode, Mode,
	},
	process::{mem_space::copy::SyscallPtr, Process},
	syscall::Args,
	time::unit::{TimeUnit, Timespec, TimestampScale},
};
use core::ffi::{c_int, c_long};
use utils::{
	errno,
	errno::{EResult, Errno},
	lock::Mutex,
	ptr::arc::Arc,
};

// TODO Check types
/// A file's stat.
#[repr(C)]
#[derive(Debug)]
pub struct Stat {
	/// ID of the device containing the file.
	st_dev: u64,

	/// Padding.
	__st_dev_padding: c_int,

	/// The inode number.
	st_ino: INode,
	/// File's mode.
	st_mode: Mode,
	/// Number of hard links to the file.
	st_nlink: u32,
	/// File's owner UID.
	st_uid: Uid,
	/// File's owner GID.
	st_gid: Gid,
	/// Device ID (if device file).
	st_rdev: u64,

	/// Padding.
	__st_rdev_padding: c_int,

	/// Size of the file in bytes.
	st_size: u32,
	/// Size of a block on the file's storage medium.
	st_blksize: c_long,
	/// Size of the file in blocks.
	st_blocks: u64,

	/// Timestamp of last access.
	st_atim: Timespec,
	/// Timestamp of last modification of the content.
	st_mtim: Timespec,
	/// Timestamp of last modification of the metadata.
	st_ctim: Timespec,
}

pub fn fstat64(
	Args((fd, statbuf)): Args<(c_int, SyscallPtr<Stat>)>,
	fds: Arc<Mutex<FileDescriptorTable>>,
) -> EResult<usize> {
	let file = fds.lock().get_fd(fd)?.get_file().vfs_entry.clone();
	let stat = file.get_stat()?;
	let rdev = makedev(stat.dev_major, stat.dev_minor);
	let stat = Stat {
		st_dev: 0, // TODO

		__st_dev_padding: 0,

		st_ino: file.location.inode,
		st_mode: stat.mode,
		st_nlink: stat.nlink as _,
		st_uid: stat.uid,
		st_gid: stat.gid,
		st_rdev: rdev,

		__st_rdev_padding: 0,

		st_size: stat.size as _,
		st_blksize: 512, // TODO
		st_blocks: stat.blocks,

		st_atim: Timespec::from_nano(TimestampScale::convert(
			stat.atime,
			TimestampScale::Second,
			TimestampScale::Nanosecond,
		)),
		st_mtim: Timespec::from_nano(TimestampScale::convert(
			stat.mtime,
			TimestampScale::Second,
			TimestampScale::Nanosecond,
		)),
		st_ctim: Timespec::from_nano(TimestampScale::convert(
			stat.ctime,
			TimestampScale::Second,
			TimestampScale::Nanosecond,
		)),
	};
	statbuf.copy_to_user(stat)?;
	Ok(0)
}
