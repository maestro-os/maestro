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
	device::BlkDev,
	file::{
		fs::{
			downcast_fs, ext2::dirent::Dirent, FileOps, Filesystem, FilesystemOps, FilesystemType,
			NodeOps, StatSet, Statfs,
		},
		vfs,
		vfs::node::Node,
		DirContext, DirEntry, File, FileType, INode, Stat,
	},
	memory::RcPage,
	sync::mutex::Mutex,
	time::{clock, clock::CLOCK_MONOTONIC, unit::TimestampScale},
};
use bgd::BlockGroupDescriptor;
use core::{
	cmp::{max, min},
	fmt,
	fmt::Formatter,
	intrinsics::unlikely,
};
use inode::Ext2INode;
use macros::AnyRepr;
use utils::{
	boxed::Box, collections::path::PathBuf, errno, errno::EResult, limits::PAGE_SIZE, math,
	ptr::arc::Arc, vec,
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

/// Node operations.
#[derive(Debug)]
struct Ext2NodeOps;

impl NodeOps for Ext2NodeOps {
	fn set_stat(&self, node: &Node, set: &StatSet) -> EResult<()> {
		let fs = downcast_fs::<Ext2Fs>(&*node.fs.ops);
		let superblock = fs.superblock.lock();
		let mut inode_ = Ext2INode::read(node.inode as _, &superblock, &*fs.dev)?;
		if let Some(mode) = set.mode {
			inode_.set_permissions(mode);
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
		inode_.write(node.inode as _, &superblock, &*fs.dev)
	}

	fn lookup_entry<'n>(&self, dir: &Node, ent: &mut vfs::Entry) -> EResult<()> {
		let fs = downcast_fs::<Ext2Fs>(&*dir.fs.ops);
		let superblock = fs.superblock.lock();
		let inode_ = Ext2INode::read(dir.inode as _, &superblock, &*fs.dev)?;
		ent.node = inode_
			.get_dirent(&ent.name, &superblock, &*fs.dev)?
			.map(|(inode, ..)| -> EResult<_> {
				let inode_ = Ext2INode::read(inode as _, &superblock, &*fs.dev)?;
				let stat = inode_.stat(&superblock);
				let node = Arc::new(Node {
					inode: inode as _,
					fs: dir.fs.clone(),

					stat: Mutex::new(stat),

					node_ops: Box::new(Ext2NodeOps)?,
					file_ops: Box::new(Ext2FileOps)?,

					pages: Default::default(),
				})?;
				Ok(node)
			})
			.transpose()?;
		Ok(())
	}

	fn iter_entries(&self, dir: &Node, ctx: &mut DirContext) -> EResult<()> {
		let fs = downcast_fs::<Ext2Fs>(&*dir.fs.ops);
		let superblock = fs.superblock.lock();
		let inode = Ext2INode::read(dir.inode as _, &superblock, &*fs.dev)?;
		if inode.get_type() != FileType::Directory {
			return Err(errno!(ENOTDIR));
		}
		// Iterate on entries
		let blk_size = superblock.get_block_size();
		let mut buf = vec![0; blk_size as _]?;
		'outer: while ctx.off < inode.get_size(&superblock) {
			// Read content block
			let blk_off = ctx.off / blk_size as u64;
			let res = inode.translate_blk_off(blk_off as _, &superblock, &*fs.dev);
			let blk_off = match res {
				Ok(Some(o)) => o,
				// If reaching a zero block, stop
				Ok(None) => break,
				// If reaching the block limit, stop
				Err(e) if e.as_int() == errno::EOVERFLOW => break,
				Err(e) => return Err(e),
			};
			read_block(blk_off.get() as _, blk_size, &*fs.dev, &mut buf)?;
			// Read the next entry in the current block, skipping free ones
			let ent = loop {
				// Read entry
				let inner_off = (ctx.off % blk_size as u64) as usize;
				let ent = Dirent::from_slice(&mut buf[inner_off..], &superblock)?;
				// Update offset
				let prev_off = ctx.off;
				// If not free, use this entry
				if !ent.is_free() {
					break ent;
				}
				ctx.off += ent.rec_len as u64;
				// If the next entry is on another block, read next block
				if (prev_off / blk_size as u64) != (ctx.off / blk_size as u64) {
					continue 'outer;
				}
			};
			let e = DirEntry {
				inode: ent.inode as _,
				entry_type: ent.get_type(&superblock, &*fs.dev)?,
				name: ent.get_name(&superblock),
			};
			if !(ctx.write)(&e)? {
				break;
			}
			ctx.off += ent.rec_len as u64;
		}
		Ok(())
	}

	fn link(&self, parent: &Node, ent: &vfs::Entry) -> EResult<()> {
		let fs = downcast_fs::<Ext2Fs>(&*parent.fs.ops);
		if unlikely(fs.readonly) {
			return Err(errno!(EROFS));
		}
		let mut superblock = fs.superblock.lock();
		let target = ent.node();
		// Parent inode
		let mut parent_inode = Ext2INode::read(parent.inode as _, &superblock, &*fs.dev)?;
		// Check the parent file is a directory
		if parent_inode.get_type() != FileType::Directory {
			return Err(errno!(ENOTDIR));
		}
		// Check the entry does not exist
		if parent_inode
			.get_dirent(&ent.name, &superblock, &*fs.dev)?
			.is_some()
		{
			return Err(errno!(EEXIST));
		}
		let mut target_inode = Ext2INode::read(target.inode, &superblock, &*fs.dev)?;
		if unlikely(target_inode.i_links_count == u16::MAX) {
			return Err(errno!(EMFILE));
		}
		if target_inode.get_type() == FileType::Directory {
			if unlikely(parent_inode.i_links_count == u16::MAX) {
				return Err(errno!(EMFILE));
			}
			// Create the `..` entry
			target_inode.add_dirent(
				&mut superblock,
				&*fs.dev,
				parent.inode as _,
				b"..",
				FileType::Directory,
			)?;
			parent_inode.i_links_count += 1;
			parent.stat.lock().nlink = parent_inode.i_links_count;
		}
		// Create entry
		parent_inode.add_dirent(
			&mut superblock,
			&*fs.dev,
			target.inode as _,
			&ent.name,
			target_inode.get_type(),
		)?;
		target_inode.i_links_count += 1;
		target.stat.lock().nlink = target_inode.i_links_count;
		// Write
		parent_inode.write(parent.inode as _, &superblock, &*fs.dev)?;
		target_inode.write(target.inode, &superblock, &*fs.dev)?;
		Ok(())
	}

	fn unlink(&self, parent: &Node, ent: &vfs::Entry) -> EResult<()> {
		let fs = downcast_fs::<Ext2Fs>(&*parent.fs.ops);
		if unlikely(fs.readonly) {
			return Err(errno!(EROFS));
		}
		if ent.name == "." || ent.name == ".." {
			return Err(errno!(EINVAL));
		}
		let mut superblock = fs.superblock.lock();
		// The parent inode
		let mut parent_ = Ext2INode::read(parent.inode as _, &superblock, &*fs.dev)?;
		// Check the parent file is a directory
		if parent_.get_type() != FileType::Directory {
			return Err(errno!(ENOTDIR));
		}
		// The inode number and the offset of the entry
		let (remove_inode, _, remove_off) = parent_
			.get_dirent(&ent.name, &superblock, &*fs.dev)?
			.ok_or_else(|| errno!(ENOENT))?;
		let mut target = Ext2INode::read(remove_inode as _, &superblock, &*fs.dev)?;
		if target.get_type() == FileType::Directory {
			// If the directory is not empty, error
			if !target.is_directory_empty(&superblock, &*fs.dev)? {
				return Err(errno!(ENOTEMPTY));
			}
			// Decrement links because of the `..` entry being removed
			parent_.i_links_count = parent_.i_links_count.saturating_sub(1);
			parent.stat.lock().nlink = parent_.i_links_count;
		}
		// Decrement the hard links count
		target.i_links_count = target.i_links_count.saturating_sub(1);
		ent.node().stat.lock().nlink = target.i_links_count;
		// Write
		target.write(remove_inode as _, &superblock, &*fs.dev)?;
		// Remove the directory entry
		parent_.remove_dirent(remove_off, &mut superblock, &*fs.dev)?;
		parent_.write(parent.inode as _, &superblock, &*fs.dev)?;
		Ok(())
	}

	fn readlink(&self, node: &Node, buf: &mut [u8]) -> EResult<usize> {
		let fs = downcast_fs::<Ext2Fs>(&*node.fs.ops);
		let superblock = fs.superblock.lock();
		let inode_ = Ext2INode::read(node.inode as _, &superblock, &*fs.dev)?;
		if inode_.get_type() != FileType::Link {
			return Err(errno!(EINVAL));
		}
		inode_.read_link(&superblock, &fs.dev, buf)
	}

	fn writelink(&self, node: &Node, buf: &[u8]) -> EResult<()> {
		let fs = downcast_fs::<Ext2Fs>(&*node.fs.ops);
		let mut superblock = fs.superblock.lock();
		let mut inode_ = Ext2INode::read(node.inode as _, &superblock, &fs.dev)?;
		if inode_.get_type() != FileType::Link {
			return Err(errno!(EINVAL));
		}
		inode_.write_link(&mut superblock, &fs.dev, buf)?;
		// Update status
		node.stat.lock().size = buf.len() as _;
		Ok(())
	}

	fn rename(
		&self,
		_old_parent: &Node,
		_old_name: &vfs::Entry,
		_new_parent: &Node,
		_new_name: &vfs::Entry,
	) -> EResult<()> {
		todo!()
	}

	fn readahead(&self, node: &Node, off: u64) -> EResult<RcPage> {
		// First check cache
		let mut pages = node.pages.lock();
		let cached = pages.get(&off).cloned();
		if let Some(page) = cached {
			return Ok(page);
		}
		// Cache miss: read from device and insert in cache
		let fs = downcast_fs::<Ext2Fs>(&*node.fs.ops);
		let page = fs.dev.read_page(off)?;
		pages.insert(off, page.clone())?;
		Ok(page)
	}

	fn writeback(&self, _node: &Node, _off: u64) -> EResult<()> {
		todo!()
	}
}

/// Open file operations.
#[derive(Debug)]
pub struct Ext2FileOps;

impl FileOps for Ext2FileOps {
	fn read(&self, file: &File, off: u64, buf: &mut [u8]) -> EResult<usize> {
		let node = file.node().unwrap();
		let fs = downcast_fs::<Ext2Fs>(&*node.fs.ops);
		let superblock = fs.superblock.lock();
		let inode_ = Ext2INode::read(node.inode as _, &superblock, &*fs.dev)?;
		if inode_.get_type() != FileType::Regular {
			return Err(errno!(EINVAL));
		}
		inode_.read_content(off, buf, &superblock, &*fs.dev)
	}

	fn write(&self, file: &File, off: u64, buf: &[u8]) -> EResult<usize> {
		let node = file.node().unwrap();
		let fs = downcast_fs::<Ext2Fs>(&*node.fs.ops);
		if unlikely(fs.readonly) {
			return Err(errno!(EROFS));
		}
		let mut superblock = fs.superblock.lock();
		let mut inode_ = Ext2INode::read(node.inode as _, &superblock, &*fs.dev)?;
		if inode_.get_type() != FileType::Regular {
			return Err(errno!(EINVAL));
		}
		inode_.write_content(off, buf, &mut superblock, &*fs.dev)?;
		inode_.write(node.inode as _, &superblock, &*fs.dev)?;
		superblock.write(&*fs.dev)?;
		// Update status
		node.stat.lock().size = inode_.get_size(&superblock);
		Ok(buf.len() as _)
	}

	fn truncate(&self, file: &File, size: u64) -> EResult<()> {
		let node = file.node().unwrap();
		let fs = downcast_fs::<Ext2Fs>(&*node.fs.ops);
		if unlikely(fs.readonly) {
			return Err(errno!(EROFS));
		}
		let mut superblock = fs.superblock.lock();
		let mut inode_ = Ext2INode::read(node.inode as _, &superblock, &*fs.dev)?;
		match inode_.get_type() {
			FileType::Regular => inode_.truncate(&mut superblock, &*fs.dev, size)?,
			_ => return Err(errno!(EINVAL)),
		}
		inode_.write(node.inode as _, &superblock, &*fs.dev)?;
		superblock.write(&*fs.dev)?;
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
	fn read(dev: &BlkDev) -> EResult<Self> {
		read::<Self>(SUPERBLOCK_OFFSET, SUPERBLOCK_OFFSET as _, dev)
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
	fn search_bitmap(&self, io: &BlkDev, start: u32, size: u32) -> EResult<Option<u32>> {
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

	/// Changes the state of the given entry in the given bitmap.
	///
	/// Arguments:
	/// - `start` is the starting block
	/// - `i` is the index of the entry to modify
	/// - `val` is the value to set the entry to
	///
	/// The function returns the previous value of the entry.
	fn set_bitmap(&self, dev: &BlkDev, start: u32, i: u32, val: bool) -> EResult<bool> {
		let blk_size = self.get_block_size();
		let mut buff = vec![0; blk_size as _]?;

		let bitmap_blk_index = start + (i / (blk_size * 8));
		read_block(bitmap_blk_index as _, blk_size, dev, buff.as_mut_slice())?;

		let bitmap_byte_index = i / 8;
		let bitmap_bit_index = i % 8;

		let prev = buff[bitmap_byte_index as usize] & (1 << bitmap_bit_index) != 0;
		if val {
			buff[bitmap_byte_index as usize] |= 1 << bitmap_bit_index;
		} else {
			buff[bitmap_byte_index as usize] &= !(1 << bitmap_bit_index);
		}

		write_block(bitmap_blk_index as _, blk_size, dev, buff.as_slice())?;

		Ok(prev)
	}

	/// Returns the ID of a free inode in the filesystem.
	pub fn get_free_inode(&self, dev: &BlkDev) -> EResult<u32> {
		for i in 0..self.get_block_groups_count() {
			let bgd = BlockGroupDescriptor::read(i as _, self, dev)?;
			if bgd.bg_free_inodes_count > 0 {
				if let Some(j) =
					self.search_bitmap(dev, bgd.bg_inode_bitmap, self.s_inodes_per_group)?
				{
					return Ok(i * self.s_inodes_per_group + j + 1);
				}
			}
		}

		Err(errno!(ENOSPC))
	}

	/// Marks the inode `inode` used on the filesystem.
	///
	/// `directory` tells whether the inode is allocated for a directory.
	///
	/// If the inode is already marked as used, the behaviour is undefined.
	pub fn mark_inode_used(&mut self, dev: &BlkDev, inode: u32, directory: bool) -> EResult<()> {
		if inode == 0 {
			return Ok(());
		}

		let group = (inode - 1) / self.s_inodes_per_group;
		let mut bgd = BlockGroupDescriptor::read(group, self, dev)?;

		let bitfield_index = (inode - 1) % self.s_inodes_per_group;
		let prev = self.set_bitmap(dev, bgd.bg_inode_bitmap, bitfield_index, true)?;
		if !prev {
			bgd.bg_free_inodes_count -= 1;
			if directory {
				bgd.bg_used_dirs_count += 1;
			}
			bgd.write(group, self, dev)?;

			self.s_free_inodes_count -= 1;
		}

		Ok(())
	}

	/// Marks the inode `inode` available on the filesystem.
	///
	/// If `inode` is zero, the function does nothing.
	///
	/// `directory` tells whether the inode is allocated for a directory.
	///
	/// If the inode is already marked as free, the behaviour is undefined.
	pub fn free_inode(&mut self, dev: &BlkDev, inode: INode, directory: bool) -> EResult<()> {
		let inode: u32 = inode.try_into().map_err(|_| errno!(EOVERFLOW))?;
		if inode == 0 {
			return Ok(());
		}

		let group = (inode - 1) / self.s_inodes_per_group;
		let mut bgd = BlockGroupDescriptor::read(group, self, dev)?;

		let bitfield_index = (inode - 1) % self.s_inodes_per_group;
		let prev = self.set_bitmap(dev, bgd.bg_inode_bitmap, bitfield_index, false)?;
		if prev {
			bgd.bg_free_inodes_count += 1;
			if directory {
				bgd.bg_used_dirs_count -= 1;
			}
			bgd.write(group, self, dev)?;

			self.s_free_inodes_count += 1;
		}

		Ok(())
	}

	/// Returns the ID of a free block in the filesystem.
	pub fn get_free_block(&self, dev: &BlkDev) -> EResult<u32> {
		for i in 0..self.get_block_groups_count() {
			let bgd = BlockGroupDescriptor::read(i as _, self, dev)?;
			if bgd.bg_free_blocks_count > 0 {
				if let Some(j) =
					self.search_bitmap(dev, bgd.bg_block_bitmap, self.s_blocks_per_group)?
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
	/// If `blk` is zero, the function does nothing.
	pub fn mark_block_used(&mut self, dev: &BlkDev, blk: u32) -> EResult<()> {
		if blk == 0 {
			return Ok(());
		}
		if blk <= 2 || blk >= self.s_blocks_count {
			return Err(errno!(EUCLEAN));
		}

		let group = blk / self.s_blocks_per_group;
		let mut bgd = BlockGroupDescriptor::read(group, self, dev)?;

		let bitfield_index = blk % self.s_blocks_per_group;
		let prev = self.set_bitmap(dev, bgd.bg_block_bitmap, bitfield_index, true)?;
		if !prev {
			bgd.bg_free_blocks_count -= 1;
			bgd.write(group, self, dev)?;

			self.s_free_blocks_count -= 1;
		}

		Ok(())
	}

	/// Marks the block `blk` available on the filesystem.
	///
	/// If `blk` is zero, the function does nothing.
	pub fn free_block(&mut self, dev: &BlkDev, blk: u32) -> EResult<()> {
		if blk == 0 {
			return Ok(());
		}
		if blk <= 2 || blk >= self.s_blocks_count {
			return Err(errno!(EUCLEAN));
		}

		let group = blk / self.s_blocks_per_group;
		let mut bgd = BlockGroupDescriptor::read(group, self, dev)?;

		let bitfield_index = blk % self.s_blocks_per_group;
		let prev = self.set_bitmap(dev, bgd.bg_block_bitmap, bitfield_index, false)?;
		if prev {
			bgd.bg_free_blocks_count += 1;
			bgd.write(group, self, dev)?;

			self.s_free_blocks_count += 1;
		}

		Ok(())
	}

	/// Writes the superblock on the device.
	pub fn write(&self, io: &BlkDev) -> EResult<()> {
		write(SUPERBLOCK_OFFSET, SUPERBLOCK_OFFSET as _, io, self)
	}
}

/// An instance of the ext2 filesystem.
struct Ext2Fs {
	/// The device on which the filesystem is located
	dev: Arc<BlkDev>,
	/// The filesystem's superblock
	superblock: Mutex<Superblock>,
	/// Tells whether the filesystem is mounted in read-only
	readonly: bool,
}

// TODO Update the write timestamp when the fs is written (take mount flags into
// account)
impl FilesystemOps for Ext2Fs {
	fn get_name(&self) -> &[u8] {
		b"ext2"
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

	fn root(&self, fs: Arc<Filesystem>) -> EResult<Arc<Node>> {
		Filesystem::node_get_or_insert(fs, inode::ROOT_DIRECTORY_INODE as _, || {
			let superblock = self.superblock.lock();
			// Check the inode exists
			let inode =
				Ext2INode::read(inode::ROOT_DIRECTORY_INODE as _, &superblock, &*self.dev)?;
			let stat = inode.stat(&superblock);
			Ok((stat, Box::new(Ext2NodeOps)?, Box::new(Ext2FileOps)?))
		})
	}

	fn create_node(&self, fs: Arc<Filesystem>, stat: &Stat) -> EResult<Arc<Node>> {
		if unlikely(self.readonly) {
			return Err(errno!(EROFS));
		}
		let mut superblock = self.superblock.lock();
		// Get a free inode ID
		let inode_index = superblock.get_free_inode(&*self.dev)?;
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
			i_links_count: 0,
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
		// If device, set major/minor
		let file_type = stat.get_type().ok_or_else(|| errno!(EINVAL))?;
		match file_type {
			FileType::Directory => {
				// Create the `.` entry
				inode.add_dirent(
					&mut superblock,
					&self.dev,
					inode_index,
					b".",
					FileType::Directory,
				)?;
				inode.i_links_count += 1;
			}
			FileType::BlockDevice | FileType::CharDevice => {
				inode.set_device(stat.dev_major as u8, stat.dev_minor as u8);
			}
			_ => {}
		}
		// Write node
		inode.write(inode_index as _, &superblock, &self.dev)?;
		superblock.mark_inode_used(&self.dev, inode_index, file_type == FileType::Directory)?;
		superblock.write(&self.dev)?;
		let node = Arc::new(Node {
			inode: inode_index as _,
			fs,

			stat: Mutex::new(inode.stat(&superblock)),

			node_ops: Box::new(Ext2NodeOps)?,
			file_ops: Box::new(Ext2FileOps)?,

			pages: Default::default(),
		})?;
		node.fs.node_insert(node.clone())?;
		Ok(node)
	}

	fn destroy_node(&self, node: &Node) -> EResult<()> {
		if unlikely(self.readonly) {
			return Err(errno!(EROFS));
		}
		let mut superblock = self.superblock.lock();
		let mut inode_ = Ext2INode::read(node.inode, &superblock, &self.dev)?;
		// Remove the inode
		inode_.i_links_count = 0;
		let timestamp = clock::current_time(CLOCK_MONOTONIC, TimestampScale::Second)?;
		inode_.i_dtime = timestamp as _;
		inode_.free_content(&mut superblock, &self.dev)?;
		inode_.write(node.inode, &superblock, &self.dev)?;
		// Free inode
		superblock.free_inode(
			&self.dev,
			node.inode,
			inode_.get_type() == FileType::Directory,
		)?;
		superblock.write(&self.dev)?;
		Ok(())
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

	fn detect(&self, dev: &BlkDev) -> EResult<bool> {
		Ok(Superblock::read(dev)?.is_valid())
	}

	fn load_filesystem(
		&self,
		dev: Option<Arc<BlkDev>>,
		mountpath: PathBuf,
		readonly: bool,
	) -> EResult<Box<dyn FilesystemOps>> {
		let dev = dev.ok_or_else(|| errno!(ENODEV))?;
		let mut superblock = Superblock::read(&*dev)?;
		if !superblock.is_valid() {
			return Err(errno!(EINVAL));
		}
		if (superblock.get_block_size() as usize) < PAGE_SIZE {
			return Err(errno!(EINVAL));
		}
		// Check the filesystem does not require features that are not implemented by
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
		superblock.write(&dev)?;
		Ok(Box::new(Ext2Fs {
			dev,
			superblock: Mutex::new(superblock),
			readonly,
		})?)
	}
}
