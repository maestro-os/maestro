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
			downcast_fs,
			ext2::{dirent::Dirent, inode::ROOT_DIRECTORY_INODE},
			FileOps, Filesystem, FilesystemOps, FilesystemType, NodeOps, StatSet, Statfs,
		},
		vfs,
		vfs::node::Node,
		DirContext, DirEntry, File, FileType, INode, Stat,
	},
	memory::{RcFrame, RcFrameVal},
	sync::mutex::Mutex,
	time::{clock, clock::CLOCK_MONOTONIC, unit::TimestampScale},
};
use bgd::BlockGroupDescriptor;
use core::{
	cmp::max,
	fmt,
	fmt::Formatter,
	intrinsics::unlikely,
	sync::atomic::{
		AtomicU16, AtomicU32, AtomicU8, AtomicUsize,
		Ordering::{Acquire, Relaxed, Release},
	},
};
use inode::Ext2INode;
use macros::AnyRepr;
use utils::{
	boxed::Box, collections::path::PathBuf, errno, errno::EResult, limits::PAGE_SIZE, math,
	ptr::arc::Arc,
};

// TODO Take into account user's UID/GID when allocating block/inode to handle
// reserved blocks/inodes
// TODO when accessing an inode, we need to lock the corresponding `Node` structure. This is
// currently not an issue since everything is locked by the superblock's Mutex but this very slow

/// The offset of the superblock from the beginning of the device.
const SUPERBLOCK_OFFSET: usize = 1024;
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

/// Reads the block at offset `off` from the disk.
fn read_block(dev: &BlkDev, sp: &Superblock, off: u64) -> EResult<RcFrame> {
	// cannot overflow since `s_log_block_size` is at least `2`
	let order = sp.s_log_block_size - 2;
	let page_off = off << order;
	dev.read_frame(page_off, order as _)
}

/// Finds a `0` bit in the given block, sets it atomically, then returns its offset.
///
/// If no bit is found, the function returns `None`.
fn bitmap_alloc_impl(blk: &RcFrame) -> Option<u32> {
	// Iterate on `usize` units
	const UNIT_COUNT: usize = PAGE_SIZE / size_of::<usize>();
	for unit_off in 0..UNIT_COUNT {
		let unit = &blk.slice::<AtomicUsize>()[unit_off];
		// The offset of the newly allocated entry in the unit
		let mut off = 0;
		let res = unit.fetch_update(Release, Acquire, |unit| {
			if unit != !0 {
				// Find the offset of a zero bit
				off = unit.trailing_ones();
				Some(unit | (1 << off))
			} else {
				// No bit available
				None
			}
		});
		if res.is_ok() {
			let units_off = unit_off * size_of::<usize>() * 8;
			return Some(units_off as u32 + off);
		}
	}
	None
}

/// Node operations.
#[derive(Debug)]
struct Ext2NodeOps;

impl NodeOps for Ext2NodeOps {
	fn set_stat(&self, node: &Node, set: &StatSet) -> EResult<()> {
		let fs = downcast_fs::<Ext2Fs>(&*node.fs.ops);
		let mut inode_ = Ext2INode::get(node, &fs.sp, &*fs.dev)?;
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
		Ok(())
	}

	fn lookup_entry<'n>(&self, dir: &Node, ent: &mut vfs::Entry) -> EResult<()> {
		let fs = downcast_fs::<Ext2Fs>(&*dir.fs.ops);
		let inode_ = Ext2INode::get(dir, &fs.sp, &fs.dev)?;
		ent.node = inode_
			.get_dirent(&ent.name, &fs.sp, &fs.dev)?
			.map(|(inode, ..)| -> EResult<_> {
				let mut node = Node {
					inode: inode as _,
					fs: dir.fs.clone(),

					stat: Default::default(),

					node_ops: Box::new(Ext2NodeOps)?,
					file_ops: Box::new(Ext2FileOps)?,

					lock: Default::default(),
					cache: Default::default(),
				};
				let stat = Ext2INode::get(&node, &fs.sp, &fs.dev)?.stat(&fs.sp);
				node.stat = Mutex::new(stat);
				Ok(Arc::new(node)?)
			})
			.transpose()?;
		Ok(())
	}

	fn iter_entries(&self, dir: &Node, ctx: &mut DirContext) -> EResult<()> {
		let fs = downcast_fs::<Ext2Fs>(&*dir.fs.ops);
		let inode = Ext2INode::get(dir, &fs.sp, &fs.dev)?;
		if inode.get_type() != FileType::Directory {
			return Err(errno!(ENOTDIR));
		}
		// Iterate on entries
		let blk_size = fs.sp.get_block_size();
		'outer: while ctx.off < inode.get_size(&fs.sp) {
			// Read content block
			let blk_off = ctx.off / blk_size as u64;
			let res = inode.translate_blk_off(blk_off as _, &fs.sp, &*fs.dev);
			let blk_off = match res {
				Ok(Some(o)) => o,
				// If reaching a zero block, stop
				Ok(None) => break,
				// If reaching the block limit, stop
				Err(e) if e.as_int() == errno::EOVERFLOW => break,
				Err(e) => return Err(e),
			};
			let blk = read_block(&fs.dev, &fs.sp, blk_off.get() as _)?;
			// Safe since the node is locked
			let blk_slice = unsafe { blk.slice_mut() };
			// Read the next entry in the current block, skipping free ones
			let ent = loop {
				// Read entry
				let inner_off = (ctx.off % blk_size as u64) as usize;
				let ent = Dirent::from_slice(&mut blk_slice[inner_off..], &fs.sp)?;
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
				entry_type: ent.get_type(&fs.sp),
				name: ent.get_name(&fs.sp),
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
		let target = ent.node();
		// Parent inode
		let mut parent_inode = Ext2INode::get(parent as _, &fs.sp, &*fs.dev)?;
		// Check the parent file is a directory
		if parent_inode.get_type() != FileType::Directory {
			return Err(errno!(ENOTDIR));
		}
		// Check the entry does not exist
		if parent_inode
			.get_dirent(&ent.name, &fs.sp, &fs.dev)?
			.is_some()
		{
			return Err(errno!(EEXIST));
		}
		let mut target_inode = Ext2INode::get(target, &fs.sp, &fs.dev)?;
		if unlikely(target_inode.i_links_count == u16::MAX) {
			return Err(errno!(EMFILE));
		}
		if target_inode.get_type() == FileType::Directory {
			if unlikely(parent_inode.i_links_count == u16::MAX) {
				return Err(errno!(EMFILE));
			}
			// Create the `..` entry
			target_inode.add_dirent(
				&fs.sp,
				&fs.dev,
				parent.inode as _,
				b"..",
				FileType::Directory,
			)?;
			parent_inode.i_links_count += 1;
			parent.stat.lock().nlink = parent_inode.i_links_count;
		}
		// Create entry
		parent_inode.add_dirent(
			&fs.sp,
			&fs.dev,
			target.inode as _,
			&ent.name,
			target_inode.get_type(),
		)?;
		target_inode.i_links_count += 1;
		target.stat.lock().nlink = target_inode.i_links_count;
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
		// The parent inode
		let mut parent_ = Ext2INode::get(parent, &fs.sp, &fs.dev)?;
		// Check the parent file is a directory
		if parent_.get_type() != FileType::Directory {
			return Err(errno!(ENOTDIR));
		}
		// The offset of the entry to the remove
		let (_, remove_off) = parent_
			.get_dirent(&ent.name, &fs.sp, &fs.dev)?
			.ok_or_else(|| errno!(ENOENT))?;
		let mut target = Ext2INode::get(ent.node(), &fs.sp, &fs.dev)?;
		if target.get_type() == FileType::Directory {
			// If the directory is not empty, error
			if !target.is_directory_empty(&fs.sp, &fs.dev)? {
				return Err(errno!(ENOTEMPTY));
			}
			// Decrement links because of the `..` entry being removed
			parent_.i_links_count = parent_.i_links_count.saturating_sub(1);
			parent.stat.lock().nlink = parent_.i_links_count;
		}
		// Decrement the hard links count
		target.i_links_count = target.i_links_count.saturating_sub(1);
		ent.node().stat.lock().nlink = target.i_links_count;
		// Remove the directory entry
		parent_.remove_dirent(remove_off, &fs.sp, &fs.dev)?;
		Ok(())
	}

	fn readlink(&self, node: &Node, buf: &mut [u8]) -> EResult<usize> {
		let fs = downcast_fs::<Ext2Fs>(&*node.fs.ops);
		let inode_ = Ext2INode::get(node, &fs.sp, &fs.dev)?;
		if inode_.get_type() != FileType::Link {
			return Err(errno!(EINVAL));
		}
		inode_.read_link(&fs.sp, &fs.dev, buf)
	}

	fn writelink(&self, node: &Node, buf: &[u8]) -> EResult<()> {
		let fs = downcast_fs::<Ext2Fs>(&*node.fs.ops);
		let mut inode_ = Ext2INode::get(node, &fs.sp, &fs.dev)?;
		if inode_.get_type() != FileType::Link {
			return Err(errno!(EINVAL));
		}
		inode_.write_link(&fs.sp, &fs.dev, buf)?;
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

	fn readahead(&self, node: &Node, off: u64) -> EResult<RcFrame> {
		let fs = downcast_fs::<Ext2Fs>(&*node.fs.ops);
		node.cache.get_or_insert(off, &*fs.dev.ops)
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
		let inode_ = Ext2INode::get(node, &fs.sp, &fs.dev)?;
		if inode_.get_type() != FileType::Regular {
			return Err(errno!(EINVAL));
		}
		inode_.read_content(off, buf, &fs.sp, &fs.dev)
	}

	fn write(&self, file: &File, off: u64, buf: &[u8]) -> EResult<usize> {
		let node = file.node().unwrap();
		let fs = downcast_fs::<Ext2Fs>(&*node.fs.ops);
		if unlikely(fs.readonly) {
			return Err(errno!(EROFS));
		}
		let mut inode_ = Ext2INode::get(node, &fs.sp, &fs.dev)?;
		if inode_.get_type() != FileType::Regular {
			return Err(errno!(EINVAL));
		}
		inode_.write_content(off, buf, &fs.sp, &fs.dev)?;
		// Update status
		node.stat.lock().size = inode_.get_size(&fs.sp);
		Ok(buf.len() as _)
	}

	fn truncate(&self, file: &File, size: u64) -> EResult<()> {
		let node = file.node().unwrap();
		let fs = downcast_fs::<Ext2Fs>(&*node.fs.ops);
		if unlikely(fs.readonly) {
			return Err(errno!(EROFS));
		}
		let mut inode_ = Ext2INode::get(node, &fs.sp, &fs.dev)?;
		match inode_.get_type() {
			FileType::Regular => inode_.truncate(&fs.sp, &fs.dev, size)?,
			_ => return Err(errno!(EINVAL)),
		}
		Ok(())
	}
}

/// The ext2 superblock structure.
#[repr(C)]
#[derive(AnyRepr, Debug)]
pub struct Superblock {
	/// Total number of inodes in the filesystem.
	s_inodes_count: u32,
	/// Total number of blocks in the filesystem.
	s_blocks_count: u32,
	/// Number of blocks reserved for the superuser.
	s_r_blocks_count: u32,
	/// Total number of unallocated blocks.
	s_free_blocks_count: AtomicU32,
	/// Total number of unallocated inodes.
	s_free_inodes_count: AtomicU32,
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
	s_mtime: AtomicU32,
	/// The timestamp of the last write operation.
	s_wtime: u32,
	/// The number of mounts since the last consistency check.
	s_mnt_count: AtomicU16,
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

	_padding: [u8; 788],
}

impl Superblock {
	/// Creates a new instance by reading from the given device.
	fn read(dev: &BlkDev) -> EResult<RcFrameVal<Self>> {
		let page = dev.read_frame(0)?;
		Ok(RcFrameVal::new(page, SUPERBLOCK_OFFSET))
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

	/// Finds a free element in the given bitmap, allocates it, and returns its index.
	///
	/// Arguments:
	/// - `start` is the starting block to search into
	/// - `size` is the number of elements in the bitmap
	fn bitmap_alloc(&self, dev: &BlkDev, start_blk: u32, size: u32) -> EResult<Option<u32>> {
		let blk_size = self.get_block_size();
		let end_blk = start_blk + (size / (blk_size * 8));
		// Iterate on blocks
		for blk_off in start_blk..end_blk {
			let page = read_block(dev, self, blk_off as _)?;
			if let Some(off) = bitmap_alloc_impl(&page) {
				let blk_off = blk_off - start_blk;
				return Ok(Some(blk_off * blk_size * 8 + off));
			}
		}
		Ok(None)
	}

	/// Frees the element at `index` in the bitmap starting at the block `start_blk`.
	///
	/// The function returns the previous value of the bit.
	fn bitmap_free(&self, dev: &BlkDev, start_blk: u32, index: u32) -> EResult<bool> {
		// Get block
		let blk_size = self.get_block_size();
		let blk_off = start_blk + index / (blk_size * 8);
		let page = read_block(dev, self, blk_off as _)?;
		// Atomically clear bit
		let bitmap_byte_index = index / 8;
		let byte = &page.slice::<AtomicU8>()[bitmap_byte_index as usize];
		let bitmap_bit_index = index % 8;
		let prev = byte.fetch_or(1 << bitmap_bit_index, Release);
		Ok(prev & (1 << bitmap_bit_index) != 0)
	}

	/// Allocates an inode and returns its ID.
	///
	/// `directory` tells whether the inode is allocated for a directory.
	///
	/// If no free inode can be found, the function returns an error.
	pub fn alloc_inode(&self, dev: &BlkDev, directory: bool) -> EResult<u32> {
		if unlikely(self.s_free_inodes_count.load(Acquire) == 0) {
			return Err(errno!(ENOSPC));
		}
		for group in 0..self.get_block_groups_count() {
			let bgd = BlockGroupDescriptor::get(group, self, dev)?;
			if bgd.bg_free_inodes_count.load(Acquire) == 0 {
				continue;
			}
			if let Some(j) =
				self.bitmap_alloc(dev, bgd.bg_inode_bitmap, self.s_inodes_per_group)?
			{
				self.s_free_inodes_count.fetch_sub(1, Release);
				bgd.bg_free_inodes_count.fetch_sub(1, Release);
				if directory {
					bgd.bg_used_dirs_count.fetch_add(1, Release);
				}
				return Ok(group * self.s_inodes_per_group + j + 1);
			}
		}
		Err(errno!(ENOSPC))
	}

	/// Marks the inode `inode` available on the filesystem.
	///
	/// If `inode` is zero, the function does nothing.
	///
	/// `directory` tells whether the inode is allocated for a directory.
	///
	/// If the inode is already marked as free, the behaviour is undefined.
	pub fn free_inode(&self, dev: &BlkDev, inode: INode, directory: bool) -> EResult<()> {
		// Validation
		let inode: u32 = inode.try_into().map_err(|_| errno!(EOVERFLOW))?;
		if unlikely(inode == 0) {
			return Ok(());
		}
		// Get block group
		let group = (inode - 1) / self.s_inodes_per_group;
		let bgd = BlockGroupDescriptor::get(group, self, dev)?;
		// Clear bit and update counters
		let bitfield_index = (inode - 1) % self.s_inodes_per_group;
		let prev = self.bitmap_free(dev, bgd.bg_inode_bitmap, bitfield_index)?;
		// Check to avoid overflow in case of corrupted filesystem
		if prev {
			self.s_free_inodes_count.fetch_add(1, Release);
			bgd.bg_free_inodes_count.fetch_add(1, Release);
			if directory {
				bgd.bg_used_dirs_count.fetch_sub(1, Release);
			}
		}
		Ok(())
	}

	/// Returns the ID of a free block in the filesystem.
	pub fn alloc_block(&self, dev: &BlkDev) -> EResult<u32> {
		if unlikely(self.s_free_inodes_count.load(Acquire) == 0) {
			return Err(errno!(ENOSPC));
		}
		for i in 0..self.get_block_groups_count() {
			let bgd = BlockGroupDescriptor::get(i as _, self, dev)?;
			if bgd.bg_free_blocks_count.load(Acquire) == 0 {
				continue;
			}
			let Some(j) = self.bitmap_alloc(dev, bgd.bg_block_bitmap, self.s_blocks_per_group)?
			else {
				continue;
			};
			let blk_index = i * self.s_blocks_per_group + j;
			if unlikely(blk_index <= 2 || blk_index >= self.s_blocks_count) {
				return Err(errno!(EUCLEAN));
			}
			self.s_free_blocks_count.fetch_sub(1, Release);
			bgd.bg_free_blocks_count.fetch_sub(1, Release);
			return Ok(blk_index);
		}
		Err(errno!(ENOSPC))
	}

	/// Marks the block `blk` available on the filesystem.
	///
	/// If `blk` is zero, the function does nothing.
	pub fn free_block(&self, dev: &BlkDev, blk: u32) -> EResult<()> {
		// Validation
		if unlikely(blk <= 2 || blk >= self.s_blocks_count) {
			return Err(errno!(EUCLEAN));
		}
		// Get block group
		let group = blk / self.s_blocks_per_group;
		let bgd = BlockGroupDescriptor::get(group, self, dev)?;
		// Clear bit and update counters
		let bitfield_index = blk % self.s_blocks_per_group;
		let prev = self.bitmap_free(dev, bgd.bg_block_bitmap, bitfield_index)?;
		// Check to avoid overflow in case of corrupted filesystem
		if prev {
			self.s_free_blocks_count.fetch_add(1, Release);
			bgd.bg_free_blocks_count.fetch_add(1, Release);
		}
		Ok(())
	}
}

/// An instance of the ext2 filesystem.
struct Ext2Fs {
	/// The device on which the filesystem is located
	dev: Arc<BlkDev>,
	/// The filesystem's superblock
	sp: RcFrameVal<Superblock>,
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
		Ok(Statfs {
			f_type: EXT2_MAGIC as _,
			f_bsize: self.sp.get_block_size(),
			f_blocks: self.sp.s_blocks_count as _,
			f_bfree: self.sp.s_free_blocks_count.load(Relaxed) as _,
			// TODO Subtract blocks for superuser
			f_bavail: self.sp.s_free_blocks_count.load(Relaxed) as _,
			f_files: self.sp.s_inodes_count as _,
			f_ffree: self.sp.s_free_inodes_count.load(Relaxed) as _,
			f_fsid: Default::default(),
			f_namelen: MAX_NAME_LEN as _,
			f_frsize: math::pow2(self.sp.s_log_frag_size + 10),
			f_flags: 0, // TODO
		})
	}

	fn root(&self, fs: Arc<Filesystem>) -> EResult<Arc<Node>> {
		let mut node = Node {
			inode: ROOT_DIRECTORY_INODE as _,
			fs,

			stat: Default::default(),

			node_ops: Box::new(Ext2NodeOps)?,
			file_ops: Box::new(Ext2FileOps)?,

			lock: Default::default(),
			cache: Default::default(),
		};
		let stat = Ext2INode::get(&node, &self.sp, &self.dev)?.stat(&self.sp);
		node.stat = Mutex::new(stat);
		Ok(Arc::new(node)?)
	}

	fn create_node(&self, fs: Arc<Filesystem>, stat: &Stat) -> EResult<Arc<Node>> {
		if unlikely(self.readonly) {
			return Err(errno!(EROFS));
		}
		let file_type = stat.get_type().ok_or_else(|| errno!(EINVAL))?;
		// Allocate an inode
		let inode_index = self
			.sp
			.alloc_inode(&*self.dev, file_type == FileType::Directory)?;
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
		match file_type {
			FileType::Directory => {
				// Create the `.` entry
				inode.add_dirent(&self.sp, &self.dev, inode_index, b".", FileType::Directory)?;
				inode.i_links_count += 1;
			}
			FileType::BlockDevice | FileType::CharDevice => {
				inode.set_device(stat.dev_major as u8, stat.dev_minor as u8);
			}
			_ => {}
		}
		let node = Arc::new(Node {
			inode: inode_index as _,
			fs,

			stat: Mutex::new(inode.stat(&self.sp)),

			node_ops: Box::new(Ext2NodeOps)?,
			file_ops: Box::new(Ext2FileOps)?,

			lock: Default::default(),
			cache: Default::default(),
		})?;
		Ok(node)
	}

	fn destroy_node(&self, node: &Node) -> EResult<()> {
		if unlikely(self.readonly) {
			return Err(errno!(EROFS));
		}
		let mut inode = Ext2INode::get(node, &self.sp, &self.dev)?;
		// Remove the inode
		inode.i_links_count = 0;
		let timestamp = clock::current_time(CLOCK_MONOTONIC, TimestampScale::Second)?;
		inode.i_dtime = timestamp as _;
		inode.free_content(&self.sp, &self.dev)?;
		// Free inode
		self.sp.free_inode(
			&self.dev,
			node.inode,
			inode.get_type() == FileType::Directory,
		)?;
		Ok(())
	}
}

impl fmt::Debug for Ext2Fs {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		f.debug_struct("Ext2Fs")
			.field("superblock", &*self.sp)
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
		Superblock::read(dev).map(|sp| sp.is_valid())
	}

	fn load_filesystem(
		&self,
		dev: Option<Arc<BlkDev>>,
		_mountpath: PathBuf,
		readonly: bool,
	) -> EResult<Box<dyn FilesystemOps>> {
		let dev = dev.ok_or_else(|| errno!(ENODEV))?;
		let sp = Superblock::read(&*dev)?;
		if unlikely(!sp.is_valid()) {
			return Err(errno!(EINVAL));
		}
		if unlikely(sp.s_log_block_size < 2) {
			return Err(errno!(EINVAL));
		}
		// Check the filesystem does not require features that are not implemented by
		// the driver
		if sp.s_rev_level >= 1 {
			let unsupported_required_features = REQUIRED_FEATURE_COMPRESSION
				| REQUIRED_FEATURE_JOURNAL_REPLAY
				| REQUIRED_FEATURE_JOURNAL_DEVIXE;
			if sp.s_feature_incompat & unsupported_required_features != 0 {
				// TODO Log?
				return Err(errno!(EINVAL));
			}
			let unsupported_write_features = WRITE_REQUIRED_DIRECTORY_BINARY_TREE;
			if !readonly && sp.s_feature_ro_compat & unsupported_write_features != 0 {
				// TODO Log?
				return Err(errno!(EROFS));
			}
		}
		let timestamp = clock::current_time(CLOCK_MONOTONIC, TimestampScale::Second)?;
		if unlikely(sp.s_mnt_count.load(Relaxed) >= sp.s_max_mnt_count) {
			return Err(errno!(EINVAL));
		}
		// TODO
		/*if unlikely(timestamp >= superblock.s_lastcheck + superblock.s_checkinterval) {
			return Err(errno::EINVAL);
		}*/
		// Set the last mount path
		/*let mountpath_bytes = mountpath.as_bytes();
		let len = min(mountpath_bytes.len(), sp.s_last_mounted.len());
		sp.s_last_mounted[..len].copy_from_slice(&mountpath_bytes[..len]);
		sp.s_last_mounted[len..].fill(0);*/
		// Set the last mount timestamp
		sp.s_mtime.store(timestamp as _, Relaxed);
		sp.s_mnt_count.fetch_add(1, Relaxed);
		Ok(Box::new(Ext2Fs {
			dev,
			sp,
			readonly,
		})?)
	}
}
