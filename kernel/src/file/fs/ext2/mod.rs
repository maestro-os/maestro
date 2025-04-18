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

//! The ext2 filesystem is a classical filesystem used in Unix systems.
//! It is nowadays obsolete and has been replaced by ext3 and ext4.
//!
//! The filesystem divides the storage device into several substructures:
//! - Block Group: stored in the Block Group Descriptor Table (BGDT)
//! - Block: stored inside of block groups
//! - INode: represents a file in the filesystem
//! - Directory entry: an entry stored into the inode's content
//!
//! The access to an INode's data is divided into several parts, each
//! overflowing on the next when full:
//! - Direct Block Pointers: each inode has 12 of them
//! - Singly Indirect Block Pointer: a pointer to a block dedicated to storing a list of more
//!   blocks to store the inode's data. The number of blocks it can store depends on the size of a
//!   block.
//! - Doubly Indirect Block Pointer: a pointer to a block storing pointers to Singly Indirect Block
//!   Pointers, each storing pointers to more blocks.
//! - Triply Indirect Block Pointer: a pointer to a block storing pointers to Doubly Indirect Block
//!   Pointers, each storing pointers to Singly Indirect Block Pointers, each storing pointers to
//!   more blocks.
//!
//! Since the size of a block pointer is 4 bytes, the maximum size of a file is:
//! `(12 * n) + ((n/4) * n) + ((n/4)^^2 * n) + ((n/4)^^3 * n)`
//! Where `n` is the size of a block.
//!
//! For more information, see the [specifications](https://www.nongnu.org/ext2-doc/ext2.html).

mod bgd;
mod dirent;
mod inode;

use crate::{
	device::DeviceIO,
	file::{
		fs::{downcast_fs, Filesystem, FilesystemType, NodeOps, StatSet, Statfs},
		DirEntry, FileLocation, FileType, INode, Stat,
	},
	sync::mutex::Mutex,
	time::{clock, clock::CLOCK_MONOTONIC, unit::TimestampScale},
};
use bgd::BlockGroupDescriptor;
use core::{
	cmp::{max, min},
	fmt,
	fmt::Formatter,
	intrinsics::unlikely,
	mem::size_of,
};
use inode::Ext2INode;
use macros::AnyRepr;
use utils::{
	boxed::Box,
	bytes::{as_bytes, from_bytes, AnyRepr},
	collections::path::PathBuf,
	errno,
	errno::EResult,
	math,
	ptr::{arc::Arc, cow::Cow},
	vec,
};

// TODO Take into account user's UID/GID when allocating block/inode to handle
// reserved blocks/inodes
// TODO Document when a function writes on the storage device
// TODO check for hard link count overflow/underflow before performing the actual operation

/// The offset of the superblock from the beginning of the device.
const SUPERBLOCK_OFFSET: u64 = 1024;
/// The filesystem's magic number.
const EXT2_MAGIC: u16 = 0xef53;

/// Default filesystem major version.
const DEFAULT_MAJOR: u32 = 1;
/// Default filesystem minor version.
const DEFAULT_MINOR: u16 = 1;
/// Default filesystem block size.
const DEFAULT_BLOCK_SIZE: u64 = 1024;
/// Default inode size.
const DEFAULT_INODE_SIZE: u16 = 128;
/// Default inode size.
const DEFAULT_INODES_PER_GROUP: u32 = 1024;
/// Default number of blocks per block group.
const DEFAULT_BLOCKS_PER_GROUP: u32 = 1024;
/// Default number of mounts in between each fsck.
const DEFAULT_MOUNT_COUNT_BEFORE_FSCK: u16 = 1000;
/// Default elapsed time in between each fsck in seconds.
const DEFAULT_FSCK_INTERVAL: u32 = 16070400;

/// State telling that the filesystem is clean.
const FS_STATE_CLEAN: u16 = 1;
/// State telling that the filesystem has errors.
const FS_STATE_ERROR: u16 = 2;

/// Error handle action telling to ignore it.
const ERR_ACTION_IGNORE: u16 = 1;
/// Error handle action telling to mount as read-only.
const ERR_ACTION_READ_ONLY: u16 = 2;
/// Error handle action telling to trigger a kernel panic.
const ERR_ACTION_KERNEL_PANIC: u16 = 3;

/// `s_feature_compat`: Preallocation of a specified number of blocks for each new
/// directories.
const OPTIONAL_FEATURE_DIRECTORY_PREALLOCATION: u32 = 0x1;
/// `s_feature_compat`: AFS server
const OPTIONAL_FEATURE_AFS: u32 = 0x2;
/// `s_feature_compat`: Journal
const OPTIONAL_FEATURE_JOURNAL: u32 = 0x4;
/// `s_feature_compat`: Inodes have extended attributes
const OPTIONAL_FEATURE_INODE_EXTENDED: u32 = 0x8;
/// `s_feature_compat`: Filesystem can resize itself for larger partitions
const OPTIONAL_FEATURE_RESIZE: u32 = 0x10;
/// `s_feature_compat`: Directories use hash index
const OPTIONAL_FEATURE_HASH_INDEX: u32 = 0x20;

/// `s_feature_incompat`: Compression
const REQUIRED_FEATURE_COMPRESSION: u32 = 0x1;
/// `s_feature_incompat`: Directory entries have a type field
const REQUIRED_FEATURE_DIRECTORY_TYPE: u32 = 0x2;
/// `s_feature_incompat`: Filesystem needs to replay its journal
const REQUIRED_FEATURE_JOURNAL_REPLAY: u32 = 0x4;
/// `s_feature_incompat`: Filesystem uses a journal device
const REQUIRED_FEATURE_JOURNAL_DEVIXE: u32 = 0x8;

/// `s_feature_ro_compat`: Sparse superblocks and group descriptor tables
const WRITE_REQUIRED_SPARSE_SUPERBLOCKS: u32 = 0x1;
/// `s_feature_ro_compat`: Filesystem uses a 64-bit file size
const WRITE_REQUIRED_64_BITS: u32 = 0x2;
/// `s_feature_ro_compat`: Directory contents are stored in the form of a Binary Tree.
const WRITE_REQUIRED_DIRECTORY_BINARY_TREE: u32 = 0x4;

/// The maximum length of a name in the filesystem.
const MAX_NAME_LEN: usize = 255;

// TODO page cache on read_block and write_block

/// Reads the `off`th block on the given device and writes the data onto the
/// given buffer.
///
/// Arguments:
/// - `off` is the offset of the block on the device.
/// - `blk_size` is the size of a block in the filesystem.
/// - `io` is the I/O interface of the device.
/// - `buf` is the buffer to write the data on.
///
/// If the block is outside the storage's bounds, the function returns an
/// error.
fn read_block(off: u32, blk_size: u32, io: &dyn DeviceIO, buf: &mut [u8]) -> EResult<()> {
	let dev_blk_size = io.block_size().get();
	let off = off as u64 * (blk_size as u64 / dev_blk_size);
	io.read(off, buf)?;
	Ok(())
}

/// Writes the `off`th block on the given device, reading the data onto the
/// given buffer.
///
/// Arguments:
/// - `off` is the offset of the block on the device.
/// - `blk_size` is the size of a block in the filesystem.
/// - `io` is the I/O interface of the device.
/// - `buf` is the buffer to read from.
///
/// If the block is outside the storage's bounds, the function returns an
/// error.
fn write_block(off: u32, blk_size: u32, io: &dyn DeviceIO, buf: &[u8]) -> EResult<()> {
	let dev_blk_size = io.block_size().get();
	let off = off as u64 * (blk_size as u64 / dev_blk_size);
	io.write(off, buf)?;
	Ok(())
}

/// Reads an object of the given type on the given device.
///
/// Arguments:
/// - `off` is the offset in bytes on the device
/// - `blk_size` is the size of a block in the filesystem
/// - `io` is the I/O interface of the device
///
/// If the object spans several blocks, the function returns [`EUCLEAN`].
fn read<T: AnyRepr + Clone>(off: u64, blk_size: u32, io: &dyn DeviceIO) -> EResult<T> {
	let blk = off / blk_size as u64;
	let inner_off = (off % blk_size as u64) as usize;
	let mut buf = vec![0u8; blk_size as usize]?;
	read_block(blk as _, blk_size, io, &mut buf)?;
	from_bytes(&buf[inner_off..])
		.cloned()
		.ok_or_else(|| errno!(EUCLEAN))
}

/// Writes an object of the given type on the given device.
///
/// Arguments:
/// - `off` is the offset in bytes on the device
/// - `blk_size` is the size of a block in the filesystem
/// - `io` is the I/O interface of the device
/// - `val` is the value to write
///
/// If the object spans several blocks, the function returns [`EUCLEAN`].
fn write<T>(off: u64, blk_size: u32, io: &dyn DeviceIO, val: &T) -> EResult<()> {
	let len = size_of::<T>();
	let blk = off / blk_size as u64;
	let inner_off = (off % blk_size as u64) as usize;
	// Validation
	if inner_off + len > blk_size as usize {
		return Err(errno!(EUCLEAN));
	}
	// Read block
	let mut buf = vec![0u8; blk_size as usize]?;
	read_block(blk as _, blk_size, io, &mut buf)?;
	// Write back
	buf[inner_off..(inner_off + len)].copy_from_slice(as_bytes(val));
	write_block(blk as _, blk_size, io, &buf)
}

/// File operations.
#[derive(Debug)]
struct Ext2NodeOps;

impl NodeOps for Ext2NodeOps {
	fn get_stat(&self, loc: &FileLocation) -> EResult<Stat> {
		let fs = loc.get_filesystem().unwrap();
		let fs = downcast_fs::<Ext2Fs>(&*fs);
		let superblock = fs.superblock.lock();
		let inode_ = Ext2INode::read(loc.inode as _, &superblock, &*fs.io)?;
		let (dev_major, dev_minor) = inode_.get_device();
		Ok(Stat {
			mode: inode_.i_mode as _,
			nlink: inode_.i_links_count as _,
			uid: inode_.i_uid,
			gid: inode_.i_gid,
			size: inode_.get_size(&superblock),
			blocks: inode_.i_blocks as _,
			dev_major: dev_major as _,
			dev_minor: dev_minor as _,
			ctime: inode_.i_ctime as _,
			mtime: inode_.i_mtime as _,
			atime: inode_.i_atime as _,
		})
	}

	fn set_stat(&self, loc: &FileLocation, set: StatSet) -> EResult<()> {
		let fs = loc.get_filesystem().unwrap();
		let fs = downcast_fs::<Ext2Fs>(&*fs);
		let superblock = fs.superblock.lock();
		let mut inode_ = Ext2INode::read(loc.inode as _, &superblock, &*fs.io)?;
		if let Some(mode) = set.mode {
			inode_.set_permissions(mode);
		}
		if let Some(nlink) = set.nlink {
			inode_.i_links_count = nlink;
		}
		if let Some(uid) = set.uid {
			inode_.i_uid = uid;
		}
		if let Some(gid) = set.gid {
			inode_.i_gid = gid;
		}
		if let Some(ctime) = set.ctime {
			inode_.i_ctime = ctime as _;
		}
		if let Some(mtime) = set.mtime {
			inode_.i_mtime = mtime as _;
		}
		if let Some(atime) = set.atime {
			inode_.i_atime = atime as _;
		}
		inode_.write(loc.inode as _, &superblock, &*fs.io)
	}

	fn read_content(&self, loc: &FileLocation, off: u64, buf: &mut [u8]) -> EResult<usize> {
		let fs = loc.get_filesystem().unwrap();
		let fs = downcast_fs::<Ext2Fs>(&*fs);
		let superblock = fs.superblock.lock();
		let inode_ = Ext2INode::read(loc.inode as _, &superblock, &*fs.io)?;
		match inode_.get_type() {
			FileType::Regular => inode_.read_content(off, buf, &superblock, &*fs.io),
			FileType::Link => inode_.read_link(&superblock, &*fs.io, off, buf),
			_ => Err(errno!(EINVAL)),
		}
	}

	fn write_content(&self, loc: &FileLocation, off: u64, buf: &[u8]) -> EResult<usize> {
		let fs = loc.get_filesystem().unwrap();
		let fs = downcast_fs::<Ext2Fs>(&*fs);
		if unlikely(fs.readonly) {
			return Err(errno!(EROFS));
		}
		let mut superblock = fs.superblock.lock();
		let mut inode_ = Ext2INode::read(loc.inode as _, &superblock, &*fs.io)?;
		match inode_.get_type() {
			FileType::Regular => inode_.write_content(off, buf, &mut superblock, &*fs.io)?,
			FileType::Link => inode_.write_link(&mut superblock, &*fs.io, buf)?,
			_ => return Err(errno!(EINVAL)),
		}
		inode_.write(loc.inode as _, &superblock, &*fs.io)?;
		superblock.write(&*fs.io)?;
		Ok(buf.len() as _)
	}

	fn truncate_content(&self, loc: &FileLocation, size: u64) -> EResult<()> {
		let fs = loc.get_filesystem().unwrap();
		let fs = downcast_fs::<Ext2Fs>(&*fs);
		if unlikely(fs.readonly) {
			return Err(errno!(EROFS));
		}
		let fs = downcast_fs::<Ext2Fs>(fs);
		let mut superblock = fs.superblock.lock();
		let mut inode_ = Ext2INode::read(loc.inode as _, &superblock, &*fs.io)?;
		match inode_.get_type() {
			FileType::Regular => inode_.truncate(&mut superblock, &*fs.io, size)?,
			_ => return Err(errno!(EINVAL)),
		}
		inode_.write(loc.inode as _, &superblock, &*fs.io)?;
		superblock.write(&*fs.io)?;
		Ok(())
	}

	fn entry_by_name<'n>(
		&self,
		loc: &FileLocation,
		name: &'n [u8],
	) -> EResult<Option<(DirEntry<'n>, Box<dyn NodeOps>)>> {
		let fs = loc.get_filesystem().unwrap();
		let fs = downcast_fs::<Ext2Fs>(&*fs);
		let superblock = fs.superblock.lock();
		let inode_ = Ext2INode::read(loc.inode as _, &superblock, &*fs.io)?;
		let Some((inode, entry_type, _)) = inode_.get_dirent(name, &superblock, &*fs.io)? else {
			return Ok(None);
		};
		let ent = DirEntry {
			inode: inode as _,
			entry_type,
			name: Cow::Borrowed(name),
		};
		Ok(Some((ent, Box::new(Ext2NodeOps)?)))
	}

	fn next_entry(
		&self,
		loc: &FileLocation,
		off: u64,
	) -> EResult<Option<(DirEntry<'static>, u64)>> {
		let fs = loc.get_filesystem().unwrap();
		let fs = downcast_fs::<Ext2Fs>(&*fs);
		let superblock = fs.superblock.lock();
		let inode_ = Ext2INode::read(loc.inode as _, &superblock, &*fs.io)?;
		inode_.next_dirent(off, &superblock, &*fs.io)
	}

	fn add_file(
		&self,
		parent: &FileLocation,
		name: &[u8],
		stat: Stat,
	) -> EResult<(INode, Box<dyn NodeOps>)> {
		let fs = parent.get_filesystem().unwrap();
		let fs = downcast_fs::<Ext2Fs>(&*fs);
		if unlikely(fs.readonly) {
			return Err(errno!(EROFS));
		}
		let file_type = stat.get_type().ok_or_else(|| errno!(EINVAL))?;
		let ops = Box::new(Ext2NodeOps)?;
		let fs = downcast_fs::<Ext2Fs>(fs);
		let mut superblock = fs.superblock.lock();
		// Get parent directory
		let mut parent_ = Ext2INode::read(parent.inode as _, &superblock, &*fs.io)?;
		// Check the parent is a directory
		if parent_.get_type() != FileType::Directory {
			return Err(errno!(ENOTDIR));
		}
		// Check whether the file already exists
		if parent_.get_dirent(name, &superblock, &*fs.io)?.is_some() {
			return Err(errno!(EEXIST));
		}
		// Get a free inode ID
		let inode_index = superblock.get_free_inode(&*fs.io)?;
		// Create inode
		let mut inode = Ext2INode {
			i_mode: stat.mode as _,
			i_uid: stat.uid,
			i_size: 0,
			i_ctime: stat.ctime as _,
			i_mtime: stat.mtime as _,
			i_atime: stat.atime as _,
			i_dtime: 0,
			i_gid: stat.gid,
			i_links_count: 1,
			i_blocks: 0,
			i_flags: 0,
			i_osd1: 0,
			i_block: [0; inode::DIRECT_BLOCKS_COUNT + 3],
			i_generation: 0,
			i_file_acl: 0,
			i_dir_acl: 0,
			i_faddr: 0,
			i_osd2: [0; 12],
		};
		// Update inode with content
		match file_type {
			FileType::Directory => {
				// Add `.` and `..` entries
				inode.add_dirent(
					&mut superblock,
					&*fs.io,
					inode_index,
					b".",
					FileType::Directory,
				)?;
				inode.add_dirent(
					&mut superblock,
					&*fs.io,
					parent.inode as _,
					b"..",
					FileType::Directory,
				)?;
				inode.i_links_count += 1;
				parent_.i_links_count += 1;
			}
			FileType::BlockDevice | FileType::CharDevice => {
				if stat.dev_major > (u8::MAX as u32) || stat.dev_minor > (u8::MAX as u32) {
					return Err(errno!(ENODEV));
				}
				inode.set_device(stat.dev_major as u8, stat.dev_minor as u8);
			}
			_ => {}
		}
		let is_dir = file_type == FileType::Directory;
		// Write node
		inode.write(inode_index as _, &superblock, &*fs.io)?;
		superblock.mark_inode_used(&*fs.io, inode_index, is_dir)?;
		superblock.write(&*fs.io)?;
		// Write parent
		parent_.add_dirent(&mut superblock, &*fs.io, inode_index, name, file_type)?;
		parent_.write(parent.inode as _, &superblock, &*fs.io)?;
		Ok((inode_index as _, ops))
	}

	fn link(&self, parent: &FileLocation, name: &[u8], target: INode) -> EResult<()> {
		let fs = parent.get_filesystem().unwrap();
		let fs = downcast_fs::<Ext2Fs>(&*fs);
		if unlikely(fs.readonly) {
			return Err(errno!(EROFS));
		}
		let mut superblock = fs.superblock.lock();
		// Parent inode
		let mut parent_ = Ext2INode::read(parent.inode as _, &superblock, &*fs.io)?;
		// Check the parent file is a directory
		if parent_.get_type() != FileType::Directory {
			return Err(errno!(ENOTDIR));
		}
		// Check the entry doesn't exist
		if parent_.get_dirent(name, &superblock, &*fs.io)?.is_some() {
			return Err(errno!(EEXIST));
		}
		// The inode
		let mut inode_ = Ext2INode::read(target as _, &superblock, &*fs.io)?;
		// Check the maximum number of links is not exceeded
		if inode_.i_links_count == u16::MAX {
			return Err(errno!(EMFILE));
		}
		if inode_.get_type() == FileType::Directory {
			// Cannot add hard links to directories
			return Err(errno!(EISDIR));
		}
		// Update links count
		inode_.i_links_count += 1;
		// Write directory entry
		parent_.add_dirent(
			&mut superblock,
			&*fs.io,
			target as _,
			name,
			inode_.get_type(),
		)?;
		parent_.write(parent.inode as _, &superblock, &*fs.io)?;
		inode_.write(target as _, &superblock, &*fs.io)?;
		Ok(())
	}

	fn unlink(&self, parent: &FileLocation, name: &[u8]) -> EResult<()> {
		let fs = parent.get_filesystem().unwrap();
		let fs = downcast_fs::<Ext2Fs>(&*fs);
		if unlikely(fs.readonly) {
			return Err(errno!(EROFS));
		}
		if name == b"." || name == b".." {
			return Err(errno!(EINVAL));
		}
		let mut superblock = fs.superblock.lock();
		// The parent inode
		let mut parent_ = Ext2INode::read(parent.inode as _, &superblock, &*fs.io)?;
		// Check the parent file is a directory
		if parent_.get_type() != FileType::Directory {
			return Err(errno!(ENOTDIR));
		}
		// The inode number and the offset of the entry
		let (remove_inode, _, remove_off) = parent_
			.get_dirent(name, &superblock, &*fs.io)?
			.ok_or_else(|| errno!(ENOENT))?;
		let mut remove_inode_ = Ext2INode::read(remove_inode as _, &superblock, &*fs.io)?;
		if remove_inode_.get_type() == FileType::Directory {
			// If the directory is not empty, error
			if !remove_inode_.is_directory_empty(&superblock, &*fs.io)? {
				return Err(errno!(ENOTEMPTY));
			}
			// Decrement links because of the `..` entry being removed
			parent_.i_links_count = parent_.i_links_count.saturating_sub(1);
		}
		// Decrement the hard links count
		remove_inode_.i_links_count = remove_inode_.i_links_count.saturating_sub(1);
		remove_inode_.write(remove_inode as _, &superblock, &*fs.io)?;
		// Remove the directory entry
		parent_.remove_dirent(remove_off, &mut superblock, &*fs.io)?;
		parent_.write(parent.inode as _, &superblock, &*fs.io)?;
		Ok(())
	}

	fn remove_node(&self, loc: &FileLocation) -> EResult<()> {
		let fs = loc.get_filesystem().unwrap();
		let fs = downcast_fs::<Ext2Fs>(&*fs);
		if unlikely(fs.readonly) {
			return Err(errno!(EROFS));
		}
		let mut superblock = fs.superblock.lock();
		let mut inode_ = Ext2INode::read(loc.inode, &superblock, &*fs.io)?;
		// Remove the inode
		inode_.i_links_count = 0;
		let timestamp = clock::current_time(CLOCK_MONOTONIC, TimestampScale::Second)?;
		inode_.i_dtime = timestamp as _;
		inode_.free_content(&mut superblock, &*fs.io)?;
		inode_.write(loc.inode, &superblock, &*fs.io)?;
		// Free inode
		superblock.free_inode(&*fs.io, loc.inode, inode_.get_type() == FileType::Directory)?;
		superblock.write(&*fs.io)?;
		Ok(())
	}
}

/// The ext2 superblock structure.
#[repr(C)]
#[derive(AnyRepr, Clone, Debug)]
pub struct Superblock {
	/// Total number of inodes in the filesystem.
	s_inodes_count: u32,
	/// Total number of blocks in the filesystem.
	s_blocks_count: u32,
	/// Number of blocks reserved for the superuser.
	s_r_blocks_count: u32,
	/// Total number of unallocated blocks.
	s_free_blocks_count: u32,
	/// Total number of unallocated inodes.
	s_free_inodes_count: u32,
	/// Block number of the block containing the superblock.
	s_first_data_block: u32,
	/// `log2(block_size) - 10`
	s_log_block_size: u32,
	/// `log2(fragment_size) - 10`
	s_log_frag_size: u32,
	/// The number of blocks per block group.
	s_blocks_per_group: u32,
	/// The number of fragments per block group.
	s_frags_per_group: u32,
	/// The number of inodes per block group.
	s_inodes_per_group: u32,
	/// The timestamp of the last mount operation.
	s_mtime: u32,
	/// The timestamp of the last write operation.
	s_wtime: u32,
	/// The number of mounts since the last consistency check.
	s_mnt_count: u16,
	/// The number of mounts allowed before a consistency check must be done.
	s_max_mnt_count: u16,
	/// The ext2 signature.
	s_magic: u16,
	/// The filesystem's state.
	s_state: u16,
	/// The action to perform when an error is detected.
	s_errors: u16,
	/// The minor version.
	s_minor_rev_level: u16,
	/// The timestamp of the last consistency check.
	s_lastcheck: u32,
	/// The interval between mandatory consistency checks.
	s_checkinterval: u32,
	/// The id os the operating system from which the filesystem was created.
	s_creator_os: u32,
	/// The major version.
	s_rev_level: u32,
	/// The UID of the user that can use reserved blocks.
	s_def_resuid: u16,
	/// The GID of the group that can use reserved blocks.
	s_def_resgid: u16,

	// Extended superblock fields
	/// The first non reserved inode
	s_first_ino: u32,
	/// The size of the inode structure in bytes.
	s_inode_size: u16,
	/// The block group containing the superblock.
	s_block_group_nr: u16,
	/// Optional features for the implementation to support.
	s_feature_compat: u32,
	/// Required features for the implementation to support.
	s_feature_incompat: u32,
	/// Required features for the implementation to support for writing.
	s_feature_ro_compat: u32,
	/// The filesystem id.
	s_uuid: [u8; 16],
	/// The volume name.
	s_volume_name: [u8; 16],
	/// The path the volume was last mounted to.
	s_last_mounted: [u8; 64],
	/// Used compression algorithms.
	s_algo_bitmap: u32,
	/// The number of blocks to preallocate for files.
	s_prealloc_blocks: u8,
	/// The number of blocks to preallocate for directories.
	s_prealloc_dir_blocks: u8,
	/// Unused.
	_pad: u16,
	/// The journal ID.
	s_journal_uuid: [u8; 16],
	/// The journal inode.
	s_journal_inum: u32,
	/// The journal device.
	s_journal_dev: u32,
	/// The head of orphan inodes list.
	s_last_orphan: u32,

	/// Structure padding.
	_padding: [u8; 788],
}

impl Superblock {
	/// Creates a new instance by reading from the given device.
	fn read(io: &dyn DeviceIO) -> EResult<Self> {
		read::<Self>(SUPERBLOCK_OFFSET, SUPERBLOCK_OFFSET as _, io)
	}

	/// Tells whether the superblock is valid.
	pub fn is_valid(&self) -> bool {
		self.s_magic == EXT2_MAGIC
	}

	/// Returns the size of a block.
	pub fn get_block_size(&self) -> u32 {
		math::pow2(self.s_log_block_size + 10) as _
	}

	/// Returns the log2 of the number of block entries in each block.
	pub fn get_entries_per_block_log(&self) -> u32 {
		// An entry is 4 bytes long (`log2(4) = 2`)
		self.s_log_block_size + 10 - 2
	}

	/// Returns the block offset of the Block Group Descriptor Table.
	pub fn get_bgdt_offset(&self) -> u64 {
		(SUPERBLOCK_OFFSET / self.get_block_size() as u64) + 1
	}

	/// Returns the number of block groups.
	fn get_block_groups_count(&self) -> u32 {
		self.s_blocks_count / self.s_blocks_per_group
	}

	/// Returns the size of a fragment.
	pub fn get_fragment_size(&self) -> usize {
		math::pow2(self.s_log_frag_size + 10) as _
	}

	/// Returns the size of an inode.
	pub fn get_inode_size(&self) -> usize {
		if self.s_rev_level >= 1 {
			self.s_inode_size as _
		} else {
			128
		}
	}

	/// Returns the first inode that isn't reserved.
	pub fn get_first_available_inode(&self) -> u32 {
		if self.s_rev_level >= 1 {
			max(self.s_first_ino, inode::ROOT_DIRECTORY_INODE + 1)
		} else {
			10
		}
	}

	/// Searches in the given bitmap block `bitmap` for the first element that
	/// is not set.
	///
	/// The function returns the index to the element.
	///
	/// If every element is set, the function returns `None`.
	fn search_bitmap_blk(bitmap: &[u8]) -> Option<u32> {
		for (i, b) in bitmap.iter().enumerate() {
			if *b == 0xff {
				continue;
			}

			for j in 0..8 {
				if (*b >> j) & 0b1 == 0 {
					return Some((i * 8 + j) as _);
				}
			}
		}

		None
	}

	/// Searches into a bitmap starting at block `start`.
	///
	/// Arguments:
	/// - `io` is the I/O interface.
	/// - `start` is the starting block.
	/// - `size` is the number of entries.
	fn search_bitmap(&self, io: &dyn DeviceIO, start: u32, size: u32) -> EResult<Option<u32>> {
		let blk_size = self.get_block_size();
		let mut buff = vec![0; blk_size as _]?;
		let mut i = 0;

		while (i * (blk_size * 8)) < size {
			let bitmap_blk_index = start + i;
			read_block(bitmap_blk_index as _, blk_size, io, buff.as_mut_slice())?;

			if let Some(j) = Self::search_bitmap_blk(buff.as_slice()) {
				return Ok(Some(i * (blk_size * 8) + j));
			}

			i += 1;
		}

		Ok(None)
	}

	/// Changes the state of the given entry in the the given bitmap.
	///
	/// Arguments:
	/// - `io` is the I/O interface.
	/// - `start` is the starting block.
	/// - `i` is the index of the entry to modify.
	/// - `val` is the value to set the entry to.
	///
	/// The function returns the previous value of the entry.
	fn set_bitmap(&self, io: &dyn DeviceIO, start: u32, i: u32, val: bool) -> EResult<bool> {
		let blk_size = self.get_block_size();
		let mut buff = vec![0; blk_size as _]?;

		let bitmap_blk_index = start + (i / (blk_size * 8));
		read_block(bitmap_blk_index as _, blk_size, io, buff.as_mut_slice())?;

		let bitmap_byte_index = i / 8;
		let bitmap_bit_index = i % 8;

		let prev = buff[bitmap_byte_index as usize] & (1 << bitmap_bit_index) != 0;
		if val {
			buff[bitmap_byte_index as usize] |= 1 << bitmap_bit_index;
		} else {
			buff[bitmap_byte_index as usize] &= !(1 << bitmap_bit_index);
		}

		write_block(bitmap_blk_index as _, blk_size, io, buff.as_slice())?;

		Ok(prev)
	}

	/// Returns the id of a free inode in the filesystem.
	///
	/// `io` is the I/O interface.
	pub fn get_free_inode(&self, io: &dyn DeviceIO) -> EResult<u32> {
		for i in 0..self.get_block_groups_count() {
			let bgd = BlockGroupDescriptor::read(i as _, self, io)?;
			if bgd.bg_free_inodes_count > 0 {
				if let Some(j) =
					self.search_bitmap(io, bgd.bg_inode_bitmap, self.s_inodes_per_group)?
				{
					return Ok(i * self.s_inodes_per_group + j + 1);
				}
			}
		}

		Err(errno!(ENOSPC))
	}

	/// Marks the inode `inode` used on the filesystem.
	///
	/// Arguments:
	/// - `io` is the I/O interface.
	/// - `inode` is the inode number.
	/// - `directory` tells whether the inode is allocated for a directory.
	///
	/// If the inode is already marked as used, the behaviour is undefined.
	pub fn mark_inode_used(
		&mut self,
		io: &dyn DeviceIO,
		inode: u32,
		directory: bool,
	) -> EResult<()> {
		if inode == 0 {
			return Ok(());
		}

		let group = (inode - 1) / self.s_inodes_per_group;
		let mut bgd = BlockGroupDescriptor::read(group, self, io)?;

		let bitfield_index = (inode - 1) % self.s_inodes_per_group;
		let prev = self.set_bitmap(io, bgd.bg_inode_bitmap, bitfield_index, true)?;
		if !prev {
			bgd.bg_free_inodes_count -= 1;
			if directory {
				bgd.bg_used_dirs_count += 1;
			}
			bgd.write(group, self, io)?;

			self.s_free_inodes_count -= 1;
		}

		Ok(())
	}

	/// Marks the inode `inode` available on the filesystem.
	///
	/// Arguments:
	/// - `io` is the I/O interface.
	/// - `inode` is the inode number.
	/// - `directory` tells whether the inode is allocated for a directory.
	///
	/// If `inode` is zero, the function does nothing.
	///
	/// If the inode is already marked as free, the behaviour is undefined.
	pub fn free_inode(&mut self, io: &dyn DeviceIO, inode: INode, directory: bool) -> EResult<()> {
		let inode: u32 = inode.try_into().map_err(|_| errno!(EOVERFLOW))?;
		if inode == 0 {
			return Ok(());
		}

		let group = (inode - 1) / self.s_inodes_per_group;
		let mut bgd = BlockGroupDescriptor::read(group, self, io)?;

		let bitfield_index = (inode - 1) % self.s_inodes_per_group;
		let prev = self.set_bitmap(io, bgd.bg_inode_bitmap, bitfield_index, false)?;
		if prev {
			bgd.bg_free_inodes_count += 1;
			if directory {
				bgd.bg_used_dirs_count -= 1;
			}
			bgd.write(group, self, io)?;

			self.s_free_inodes_count += 1;
		}

		Ok(())
	}

	/// Returns the id of a free block in the filesystem.
	///
	/// `io` is the I/O interface.
	pub fn get_free_block(&self, io: &dyn DeviceIO) -> EResult<u32> {
		for i in 0..self.get_block_groups_count() {
			let bgd = BlockGroupDescriptor::read(i as _, self, io)?;
			if bgd.bg_free_blocks_count > 0 {
				if let Some(j) =
					self.search_bitmap(io, bgd.bg_block_bitmap, self.s_blocks_per_group)?
				{
					let blk = i * self.s_blocks_per_group + j;
					if blk > 2 && blk < self.s_blocks_count {
						return Ok(blk);
					} else {
						return Err(errno!(EUCLEAN));
					}
				}
			}
		}

		Err(errno!(ENOSPC))
	}

	/// Marks the block `blk` used on the filesystem.
	///
	/// Arguments:
	/// - `io` is the I/O interface.
	/// - `blk` is the block number.
	///
	/// If `blk` is zero, the function does nothing.
	pub fn mark_block_used(&mut self, io: &dyn DeviceIO, blk: u32) -> EResult<()> {
		if blk == 0 {
			return Ok(());
		}
		if blk <= 2 || blk >= self.s_blocks_count {
			return Err(errno!(EUCLEAN));
		}

		let group = blk / self.s_blocks_per_group;
		let mut bgd = BlockGroupDescriptor::read(group, self, io)?;

		let bitfield_index = blk % self.s_blocks_per_group;
		let prev = self.set_bitmap(io, bgd.bg_block_bitmap, bitfield_index, true)?;
		if !prev {
			bgd.bg_free_blocks_count -= 1;
			bgd.write(group, self, io)?;

			self.s_free_blocks_count -= 1;
		}

		Ok(())
	}

	/// Marks the block `blk` available on the filesystem.
	///
	/// Arguments:
	/// - `io` is the I/O interface.
	/// - `blk` is the block number.
	///
	/// If `blk` is zero, the function does nothing.
	pub fn free_block(&mut self, io: &dyn DeviceIO, blk: u32) -> EResult<()> {
		if blk == 0 {
			return Ok(());
		}
		if blk <= 2 || blk >= self.s_blocks_count {
			return Err(errno!(EUCLEAN));
		}

		let group = blk / self.s_blocks_per_group;
		let mut bgd = BlockGroupDescriptor::read(group, self, io)?;

		let bitfield_index = blk % self.s_blocks_per_group;
		let prev = self.set_bitmap(io, bgd.bg_block_bitmap, bitfield_index, false)?;
		if prev {
			bgd.bg_free_blocks_count += 1;
			bgd.write(group, self, io)?;

			self.s_free_blocks_count += 1;
		}

		Ok(())
	}

	/// Writes the superblock on the device.
	pub fn write(&self, io: &dyn DeviceIO) -> EResult<()> {
		write(SUPERBLOCK_OFFSET, SUPERBLOCK_OFFSET as _, io, self)
	}
}

/// An instance of the ext2 filesystem.
struct Ext2Fs {
	/// The I/O interface to the device.
	io: Arc<dyn DeviceIO>,
	/// The filesystem's superblock.
	superblock: Mutex<Superblock>,
	/// Tells whether the filesystem is mounted in read-only.
	readonly: bool,
}

impl Ext2Fs {
	/// Creates a new instance.
	///
	/// If the filesystem cannot be mounted, the function returns an Err.
	///
	/// Arguments:
	/// - `superblock` is the filesystem's superblock.
	/// - `io` is the I/O interface.
	/// - `mountpath` is the path on which the filesystem is mounted.
	/// - `readonly` tells whether the filesystem is mounted in read-only.
	fn new(
		mut superblock: Superblock,
		io: Arc<dyn DeviceIO>,
		mountpath: PathBuf,
		readonly: bool,
	) -> EResult<Self> {
		if !superblock.is_valid() {
			return Err(errno!(EINVAL));
		}
		// Check the filesystem doesn't require features that are not implemented by
		// the driver
		if superblock.s_rev_level >= 1 {
			// TODO Implement journal
			let unsupported_required_features = REQUIRED_FEATURE_COMPRESSION
				| REQUIRED_FEATURE_JOURNAL_REPLAY
				| REQUIRED_FEATURE_JOURNAL_DEVIXE;
			if superblock.s_feature_incompat & unsupported_required_features != 0 {
				// TODO Log?
				return Err(errno!(EINVAL));
			}
			// TODO Implement
			let unsupported_write_features = WRITE_REQUIRED_DIRECTORY_BINARY_TREE;
			if !readonly && superblock.s_feature_ro_compat & unsupported_write_features != 0 {
				// TODO Log?
				return Err(errno!(EROFS));
			}
		}
		let timestamp = clock::current_time(CLOCK_MONOTONIC, TimestampScale::Second)?;
		if superblock.s_mnt_count >= superblock.s_max_mnt_count {
			return Err(errno!(EINVAL));
		}
		// TODO
		/*if timestamp >= superblock.last_fsck_timestamp + superblock.fsck_interval {
			return Err(errno::EINVAL);
		}*/
		superblock.s_mnt_count += 1;
		// Set the last mount path
		let mountpath_bytes = mountpath.as_bytes();
		let len = min(mountpath_bytes.len(), superblock.s_last_mounted.len());
		superblock.s_last_mounted[..len].copy_from_slice(&mountpath_bytes[..len]);
		superblock.s_last_mounted[len..].fill(0);
		// Set the last mount timestamp
		superblock.s_mtime = timestamp as _;
		superblock.write(&*io)?;
		Ok(Self {
			io,
			superblock: Mutex::new(superblock),
			readonly,
		})
	}
}

// TODO Update the write timestamp when the fs is written (take mount flags into
// account)
impl Filesystem for Ext2Fs {
	fn get_name(&self) -> &[u8] {
		b"ext2"
	}

	fn use_cache(&self) -> bool {
		true
	}

	fn get_root_inode(&self) -> INode {
		inode::ROOT_DIRECTORY_INODE as _
	}

	fn get_stat(&self) -> EResult<Statfs> {
		let superblock = self.superblock.lock();
		let fragment_size = math::pow2(superblock.s_log_frag_size + 10);
		Ok(Statfs {
			f_type: EXT2_MAGIC as _,
			f_bsize: superblock.get_block_size(),
			f_blocks: superblock.s_blocks_count as _,
			f_bfree: superblock.s_free_blocks_count as _,
			// TODO Subtract blocks for superuser
			f_bavail: superblock.s_free_blocks_count as _,
			f_files: superblock.s_inodes_count as _,
			f_ffree: superblock.s_free_inodes_count as _,
			f_fsid: Default::default(),
			f_namelen: MAX_NAME_LEN as _,
			f_frsize: fragment_size,
			f_flags: 0, // TODO
		})
	}

	fn node_from_inode(&self, inode: INode) -> EResult<Box<dyn NodeOps>> {
		let superblock = self.superblock.lock();
		// Check the inode exists
		Ext2INode::read(inode as _, &superblock, &*self.io)?;
		Ok(Box::new(Ext2NodeOps)?)
	}
}

impl fmt::Debug for Ext2Fs {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		f.debug_struct("Ext2Fs")
			.field("superblock", &self.superblock)
			.field("readonly", &self.readonly)
			.finish()
	}
}

/// The ext2 filesystem type.
pub struct Ext2FsType;

impl FilesystemType for Ext2FsType {
	fn get_name(&self) -> &'static [u8] {
		b"ext2"
	}

	fn detect(&self, io: &dyn DeviceIO) -> EResult<bool> {
		Ok(Superblock::read(io)?.is_valid())
	}

	fn load_filesystem(
		&self,
		io: Option<Arc<dyn DeviceIO>>,
		mountpath: PathBuf,
		readonly: bool,
	) -> EResult<Arc<dyn Filesystem>> {
		let io = io.ok_or_else(|| errno!(ENODEV))?;
		let superblock = Superblock::read(&*io)?;
		let fs = Ext2Fs::new(superblock, io, mountpath, readonly)?;
		Ok(Arc::new(fs)? as _)
	}
}
