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

//! File and filesystem status system calls.

use crate::{
	device::id::{major, makedev, minor},
	file::{INode, Stat, fd::fd_to_file, fs::Statfs, vfs, vfs::Resolved},
	memory::user::{UserPtr, UserString},
	syscall::util::at,
};
use core::{
	ffi::{c_int, c_uint},
	hint::unlikely,
};
use utils::{errno, errno::EResult};

/// Old structure representing the status of a file.
#[derive(Debug)]
#[repr(C)]
pub struct OldStat {
	st_dev: u16,
	st_ino: u16,
	st_mode: u16,
	st_nlink: u16,
	st_uid: u16,
	st_gid: u16,
	st_rdev: u16,
	st_size: u32,
	st_atime: u32,
	st_mtime: u32,
	st_ctime: u32,
}

/// Status of a file, 32 bit version.
#[derive(Debug)]
#[repr(C)]
pub struct Stat32 {
	/// ID of the device containing the file
	st_dev: u32,
	/// The inode number
	st_ino: u32,
	/// File mode
	st_mode: u16,
	/// Link count
	st_link: u16,
	/// User ID of the file's owner
	st_uid: u16,
	/// Group ID of the file's group
	st_gid: u16,
	/// Device ID (if device file)
	st_rdev: u32,
	/// Size of file, in bytes
	st_size: u32,
	/// Optimal block size for I/O
	st_blksize: u32,
	/// Number of 512-byte blocks allocated
	st_blocks: u32,
	/// Timestamp of last access (seconds)
	st_atime: u32,
	/// Timestamp of last access (nanoseconds)
	st_atime_nsec: u32,
	/// Timestamp of last modification of the content (seconds)
	st_mtime: u32,
	/// Timestamp of last modification of the content (nanoseconds)
	st_mtime_nsec: u32,
	/// Timestamp of last modification of the metadata (seconds)
	st_ctime: u32,
	/// Timestamp of last modification of the metadata (nanoseconds)
	st_ctime_nsec: u32,
	/// Padding
	padding: u64,
}

/// Status of a file, 64 bit version.
#[derive(Debug)]
#[repr(C)]
pub struct Stat64 {
	/// ID of the device containing the file
	st_dev: u64,
	/// The inode number
	st_ino: u64,
	/// Number of hard links to the file
	st_nlink: u64,
	/// File mode
	st_mode: u32,
	/// User ID of the file's owner
	st_uid: u32,
	/// Group ID of the file's group
	st_gid: u32,
	/// Padding
	pad0: u32,
	/// Device ID (if device file)
	st_rdev: u64,
	/// Size of file, in bytes
	st_size: i64,
	/// Optimal block size for I/O
	st_blksize: i64,
	/// Number of 512-byte block allocated
	st_blocks: i64,
	/// Timestamp of last access (seconds)
	st_atime: u64,
	/// Timestamp of last access (nanoseconds)
	st_atime_nsec: u64,
	/// Timestamp of last modification of the content (seconds)
	st_mtime: u64,
	/// Timestamp of last modification of the content (nanoseconds)
	st_mtime_nsec: u64,
	/// Timestamp of last modification of the metadata (seconds)
	st_ctime: u64,
	/// Timestamp of last modification of the metadata (nanoseconds)
	st_ctime_nsec: u64,
}

/// Extract device number and inode from [`vfs::Entry`].
fn entry_info(entry: &vfs::Entry) -> (u64, INode) {
	let node = entry.node();
	(node.fs.dev, node.inode)
}

fn do_oldstat(stat: Stat, entry: Option<&vfs::Entry>, statbuf: UserPtr<OldStat>) -> EResult<()> {
	let (st_dev, st_ino) = entry.map(entry_info).unwrap_or_default();
	statbuf.copy_to_user(&OldStat {
		st_dev: st_dev as _,
		st_ino: st_ino as _,
		st_mode: stat.mode as _,
		st_nlink: stat.nlink as _,
		st_uid: stat.uid as _,
		st_gid: stat.gid as _,
		st_rdev: makedev(stat.dev_major, stat.dev_minor) as _,
		st_size: stat.size as _,
		st_atime: stat.atime as _,
		st_mtime: stat.mtime as _,
		st_ctime: stat.ctime as _,
	})
}

fn do_stat32(stat: Stat, entry: Option<&vfs::Entry>, statbuf: UserPtr<Stat32>) -> EResult<()> {
	let (st_dev, st_ino) = entry.map(entry_info).unwrap_or_default();
	statbuf.copy_to_user(&Stat32 {
		st_dev: st_dev as _,
		st_ino: st_ino as _,
		st_mode: stat.mode as _,
		st_link: stat.nlink as _,
		st_uid: stat.uid as _,
		st_gid: stat.gid as _,
		st_rdev: makedev(stat.dev_major, stat.dev_minor) as _,
		st_size: stat.size as _,
		st_blksize: 512, // TODO
		st_blocks: stat.blocks as _,
		st_atime: stat.atime as _,
		st_atime_nsec: 0, // TODO
		st_mtime: stat.mtime as _,
		st_mtime_nsec: 0, // TODO
		st_ctime: stat.ctime as _,
		st_ctime_nsec: 0, // TODO
		padding: 0,
	})
}

fn do_stat64(stat: Stat, entry: Option<&vfs::Entry>, statbuf: UserPtr<Stat64>) -> EResult<()> {
	let (st_dev, st_ino) = entry.map(entry_info).unwrap_or_default();
	statbuf.copy_to_user(&Stat64 {
		st_dev,
		st_ino,
		st_nlink: stat.nlink as _,
		st_mode: stat.mode as _,
		st_uid: stat.uid as _,
		st_gid: stat.gid as _,
		pad0: 0,
		st_rdev: makedev(stat.dev_major, stat.dev_minor),
		st_size: stat.size as _,
		st_blksize: 512, // TODO
		st_blocks: stat.blocks as _,
		st_atime: stat.atime,
		st_atime_nsec: 0, // TODO
		st_mtime: stat.mtime,
		st_mtime_nsec: 0, // TODO
		st_ctime: stat.ctime,
		st_ctime_nsec: 0, // TODO
	})
}

pub fn oldstat(pathname: UserString, statbuf: UserPtr<OldStat>) -> EResult<usize> {
	let pathname = pathname.copy_path_from_user()?;
	let ent = vfs::get_file_from_path(&pathname, true)?;
	do_oldstat(ent.stat(), Some(&ent), statbuf)?;
	Ok(0)
}

pub fn oldfstat(fd: c_int, statbuf: UserPtr<OldStat>) -> EResult<usize> {
	let file = fd_to_file(fd)?;
	do_oldstat(file.stat()?, file.vfs_entry.as_deref(), statbuf)?;
	Ok(0)
}

pub fn oldlstat(pathname: UserString, statbuf: UserPtr<OldStat>) -> EResult<usize> {
	let pathname = pathname.copy_path_from_user()?;
	let ent = vfs::get_file_from_path(&pathname, false)?;
	do_oldstat(ent.stat(), Some(&ent), statbuf)?;
	Ok(0)
}

pub fn stat(pathname: UserString, statbuf: UserPtr<Stat32>) -> EResult<usize> {
	let pathname = pathname.copy_path_from_user()?;
	let ent = vfs::get_file_from_path(&pathname, true)?;
	do_stat32(ent.stat(), Some(&ent), statbuf)?;
	Ok(0)
}

pub fn stat64(pathname: UserString, statbuf: UserPtr<Stat64>) -> EResult<usize> {
	let pathname = pathname.copy_path_from_user()?;
	let ent = vfs::get_file_from_path(&pathname, true)?;
	do_stat64(ent.stat(), Some(&ent), statbuf)?;
	Ok(0)
}

pub fn fstat(fd: c_int, statbuf: UserPtr<Stat32>) -> EResult<usize> {
	let file = fd_to_file(fd)?;
	do_stat32(file.stat()?, file.vfs_entry.as_deref(), statbuf)?;
	Ok(0)
}

pub fn fstat64(fd: c_int, statbuf: UserPtr<Stat64>) -> EResult<usize> {
	let file = fd_to_file(fd)?;
	do_stat64(file.stat()?, file.vfs_entry.as_deref(), statbuf)?;
	Ok(0)
}

pub fn lstat(pathname: UserString, statbuf: UserPtr<Stat32>) -> EResult<usize> {
	let pathname = pathname.copy_path_from_user()?;
	let ent = vfs::get_file_from_path(&pathname, false)?;
	do_stat32(ent.stat(), Some(&ent), statbuf)?;
	Ok(0)
}

pub fn lstat64(pathname: UserString, statbuf: UserPtr<Stat64>) -> EResult<usize> {
	let pathname = pathname.copy_path_from_user()?;
	let ent = vfs::get_file_from_path(&pathname, false)?;
	do_stat64(ent.stat(), Some(&ent), statbuf)?;
	Ok(0)
}

pub fn fstatat64(
	dirfd: c_int,
	path: UserString,
	statbuf: UserPtr<Stat64>,
	flags: c_int,
) -> EResult<usize> {
	let path = path.copy_path_opt_from_user()?;
	let Resolved::Found(ent) = at::get_file(dirfd, path.as_deref(), flags, false, true)? else {
		unreachable!();
	};
	do_stat64(ent.stat(), Some(&ent), statbuf)?;
	Ok(0)
}

pub fn newfstatat(
	dirfd: c_int,
	path: UserString,
	statbuf: UserPtr<Stat64>,
	flags: c_int,
) -> EResult<usize> {
	fstatat64(dirfd, path, statbuf, flags)
}

/// A timestamp for the [`statx`] syscall.
#[derive(Debug)]
#[repr(C)]
struct StatxTimestamp {
	/// Seconds since the Epoch (UNIX time)
	tv_sec: i64,
	/// Nanoseconds since tv_sec
	tv_nsec: u32,
	/// Reserved field.
	__reserved: i32,
}

/// Status of a file, extended.
#[derive(Debug)]
#[repr(C)]
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
	/// Memory buffer alignment for direct I/O
	stx_dio_mem_align: u32,
	/// File offset alignment for direct I/O
	stx_dio_offset_align: u32,
	/// Subvolume identifier
	stx_subvol: u64,
	/// Min atomic write unit in bytes
	stx_atomic_write_unit_min: u32,
	/// Max atomic write unit in bytes
	stx_atomic_write_unit_max: u32,
	/// Max atomic write segment count
	stx_atomic_write_segments_max: u32,
	/// Padding
	__padding1: [u32; 19],
}

pub fn statx(
	dirfd: c_int,
	pathname: UserString,
	flags: c_int,
	_mask: c_uint,
	statxbuff: UserPtr<Statx>,
) -> EResult<usize> {
	// Validation
	if unlikely(pathname.0.is_none() || statxbuff.0.is_none()) {
		return Err(errno!(EINVAL));
	}
	// TODO Implement all flags
	// Get the file
	let pathname = pathname.copy_path_opt_from_user()?;
	let Resolved::Found(file) = at::get_file(dirfd, pathname.as_deref(), flags, false, true)?
	else {
		unreachable!();
	};
	// Get file's stat
	let stat = file.stat();
	// TODO Use mask?
	// Get the major and minor numbers of the device of the file's filesystem
	let (stx_dev, stx_ino) = entry_info(&file);
	let stx_dev_minor = minor(stx_dev);
	let stx_dev_major = major(stx_dev);
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
		stx_ino,
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
		// TODO
		stx_mnt_id: 0,
		stx_dio_mem_align: 0,
		stx_dio_offset_align: 0,
		stx_subvol: 0,
		stx_atomic_write_unit_min: 0,
		stx_atomic_write_unit_max: 0,
		stx_atomic_write_segments_max: 0,
		__padding1: [0; 19],
	})?;
	Ok(0)
}

pub(super) fn do_statfs(path: UserString, buf: UserPtr<Statfs>) -> EResult<usize> {
	let path = path.copy_path_from_user()?;
	let stat = vfs::get_file_from_path(&path, false)?
		.node()
		.fs
		.ops
		.get_stat()?;
	// Write structure to userspace
	buf.copy_to_user(&stat)?;
	Ok(0)
}

pub fn statfs(path: UserString, buf: UserPtr<Statfs>) -> EResult<usize> {
	do_statfs(path, buf)
}

// TODO Check args types
pub fn statfs64(path: UserString, _sz: usize, buf: UserPtr<Statfs>) -> EResult<usize> {
	// TODO Use `sz`
	do_statfs(path, buf)
}

/// Performs the `fstatfs` system call.
pub fn do_fstatfs(fd: c_int, _sz: usize, buf: UserPtr<Statfs>) -> EResult<usize> {
	// TODO use `sz`
	let stat = fd_to_file(fd)?
		.vfs_entry
		.as_ref()
		.ok_or_else(|| errno!(ENOSYS))?
		.node()
		.fs
		.ops
		.get_stat()?;
	buf.copy_to_user(&stat)?;
	Ok(0)
}

pub fn fstatfs(fd: c_int, buf: UserPtr<Statfs>) -> EResult<usize> {
	do_fstatfs(fd, size_of::<Statfs>(), buf)
}

pub fn fstatfs64(fd: c_int, sz: usize, buf: UserPtr<Statfs>) -> EResult<usize> {
	do_fstatfs(fd, sz, buf)
}
