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

// TODO Take into account user's UID/GID when allocating block/inode to handle
// reserved blocks/inodes

mod bgd;
mod dirent;
mod inode;

use crate::{
	device::BlkDev,
	file::{
		DirContext, DirEntry, File, FileType, INode, Stat,
		fs::{
			FileOps, Filesystem, FilesystemOps, FilesystemType, NodeOps, Statfs, downcast_fs,
			ext2::{dirent::DirentIterator, inode::ROOT_DIRECTORY_INODE},
			generic_file_read, generic_file_write,
		},
		vfs,
		vfs::node::Node,
	},
	memory::{
		cache::{FrameOwner, RcFrame, RcFrameVal},
		user::UserSlice,
	},
	sync::spin::Spin,
	time::clock::{Clock, current_time_sec},
};
use bgd::BlockGroupDescriptor;
use core::{
	cmp::max,
	hint::unlikely,
	sync::atomic::{
		AtomicU8, AtomicU16, AtomicU32, AtomicUsize,
		Ordering::{Acquire, Relaxed, Release},
	},
};
use inode::Ext2INode;
use macros::AnyRepr;
use utils::{
	boxed::Box,
	bytes,
	collections::path::PathBuf,
	errno,
	errno::EResult,
	limits::{NAME_MAX, PAGE_SIZE, SYMLINK_MAX},
	math,
	ptr::arc::Arc,
};

/// The filesystem's magic number.
const EXT2_MAGIC: u16 = 0xef53;

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

/// Reads the block at offset `off` from the disk.
fn read_block(fs: &Ext2Fs, off: u64) -> EResult<RcFrame> {
	// cannot overflow since `s_log_block_size` is at least `2`
	let order = fs.sp.s_log_block_size - 2;
	let page_off = off << order;
	BlkDev::read_frame(
		&fs.dev,
		page_off,
		order as _,
		FrameOwner::BlkDev(fs.dev.clone()),
	)
}

/// Zeros the block at the given offset on the disk.
fn zero_block(fs: &Ext2Fs, off: u64) -> EResult<()> {
	let blk = read_block(fs, off)?;
	for b in blk.slice::<AtomicUsize>() {
		b.store(0, Relaxed);
	}
	blk.mark_dirty();
	Ok(())
}

/// Finds a `0` bit in the given block, sets it atomically, then returns its offset.
///
/// If no bit is found, the function returns `None`.
fn bitmap_alloc_impl(blk: &RcFrame) -> Option<u32> {
	// Iterate on `usize` units
	let unit_count = blk.len() / size_of::<usize>();
	for unit_off in 0..unit_count {
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
			blk.mark_page_dirty(unit_off / (PAGE_SIZE / size_of::<usize>()));
			let unit_off = unit_off * size_of::<usize>() * 8;
			return Some(unit_off as u32 + off);
		}
	}
	None
}

/// Node operations.
#[derive(Debug)]
struct Ext2NodeOps;

impl NodeOps for Ext2NodeOps {
	fn lookup_entry<'n>(&self, dir: &Node, ent: &mut vfs::Entry) -> EResult<()> {
		let fs = downcast_fs::<Ext2Fs>(&*dir.fs.ops);
		let inode_ = Ext2INode::get(dir, fs)?;
		ent.node = inode_
			.get_dirent(&ent.name, fs)?
			.map(|(inode, ..)| -> EResult<_> {
				dir.fs.node_get_or_insert(inode as _, || {
					let mut node = Node::new(
						inode as _,
						dir.fs.clone(),
						Default::default(),
						Box::new(Ext2NodeOps)?,
						Box::new(Ext2FileOps)?,
					);
					let stat = Ext2INode::get(&node, fs)?.stat(&fs.sp);
					node.stat = Spin::new(stat);
					Ok(Arc::new(node)?)
				})
			})
			.transpose()?;
		Ok(())
	}

	fn iter_entries(&self, dir: &Node, ctx: &mut DirContext) -> EResult<()> {
		let fs = downcast_fs::<Ext2Fs>(&*dir.fs.ops);
		let inode = Ext2INode::get(dir, fs)?;
		if inode.get_type() != FileType::Directory {
			return Err(errno!(ENOTDIR));
		}
		// Iterate on entries
		let mut blk = None;
		for ent in DirentIterator::new(fs, &inode, &mut blk, ctx.off)? {
			let (off, ent) = ent?;
			if !ent.is_free() {
				let e = DirEntry {
					inode: ent.inode as _,
					entry_type: ent.get_type(&fs.sp),
					name: ent.get_name(&fs.sp),
				};
				if !(ctx.write)(&e)? {
					break;
				}
			}
			ctx.off = off + ent.rec_len as u64;
		}
		Ok(())
	}

	fn link(&self, parent: Arc<Node>, ent: &vfs::Entry) -> EResult<()> {
		let fs = downcast_fs::<Ext2Fs>(&*parent.fs.ops);
		if unlikely(fs.readonly) {
			return Err(errno!(EROFS));
		}
		// Check the parent file is a directory
		if parent.get_type() != Some(FileType::Directory) {
			return Err(errno!(ENOTDIR));
		}
		let target = ent.node();
		// Parent inode
		let mut parent_inode = Ext2INode::get(&parent, fs)?;
		// Check the entry does not exist
		if parent_inode.get_dirent(&ent.name, fs)?.is_some() {
			return Err(errno!(EEXIST));
		}
		let mut target_inode = Ext2INode::get(target, fs)?;
		if unlikely(target_inode.i_links_count == u16::MAX) {
			return Err(errno!(EMFILE));
		}
		if target_inode.get_type() == FileType::Directory {
			if unlikely(parent_inode.i_links_count == u16::MAX) {
				return Err(errno!(EMFILE));
			}
			// Create the `..` entry
			target_inode.add_dirent(fs, parent.inode as _, b"..", FileType::Directory)?;
			parent_inode.i_links_count += 1;
			parent.stat.lock().nlink = parent_inode.i_links_count;
		}
		// Create entry
		parent_inode.add_dirent(fs, target.inode as _, &ent.name, target_inode.get_type())?;
		target_inode.i_links_count += 1;
		target.stat.lock().nlink = target_inode.i_links_count;
		parent_inode.mark_dirty();
		target_inode.mark_dirty();
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
		let mut parent_ = Ext2INode::get(parent, fs)?;
		// Check the parent file is a directory
		if parent_.get_type() != FileType::Directory {
			return Err(errno!(ENOTDIR));
		}
		// The offset of the entry to the remove
		let (_, remove_off) = parent_
			.get_dirent(&ent.name, fs)?
			.ok_or_else(|| errno!(ENOENT))?;
		let mut target = Ext2INode::get(ent.node(), fs)?;
		// Remove the directory entry
		parent_.set_dirent_inode(remove_off, 0, fs)?;
		target.i_links_count = target.i_links_count.saturating_sub(1);
		ent.node().stat.lock().nlink = target.i_links_count;
		if target.get_type() == FileType::Directory {
			// If the directory is not empty, error
			if !target.is_directory_empty(fs)? {
				return Err(errno!(ENOTEMPTY));
			}
			// Remove `..`
			if let Some((_, parent_entry_off)) = target.get_dirent(b"..", fs)? {
				target.set_dirent_inode(parent_entry_off, 0, fs)?;
				parent_.i_links_count = parent_.i_links_count.saturating_sub(1);
				parent.stat.lock().nlink = parent_.i_links_count;
			}
		}
		parent_.mark_dirty();
		target.mark_dirty();
		Ok(())
	}

	fn readlink(&self, node: &Node, buf: UserSlice<u8>) -> EResult<usize> {
		let fs = downcast_fs::<Ext2Fs>(&*node.fs.ops);
		let inode_ = Ext2INode::get(node, fs)?;
		if inode_.get_type() != FileType::Link {
			return Err(errno!(EINVAL));
		}
		let size = inode_.get_size(&fs.sp);
		if unlikely(size > SYMLINK_MAX as u64) {
			return Err(errno!(EUCLEAN));
		}
		if size <= inode::SYMLINK_INLINE_LIMIT {
			// The target is stored inline in the inode
			let src = bytes::as_bytes(&inode_.i_block);
			let len = buf.copy_to_user(0, &src[..size as usize])?;
			Ok(len)
		} else {
			// The target is stored like in regular files
			let blk =
				inode::check_blk_off(inode_.i_block[0], &fs.sp)?.ok_or_else(|| errno!(EUCLEAN))?;
			let blk = read_block(fs, blk.get() as _)?;
			let len = buf.copy_to_user(0, &blk.slice()[..size as usize])?;
			Ok(len)
		}
	}

	fn writelink(&self, node: &Node, buf: &[u8]) -> EResult<()> {
		if unlikely(buf.len() > SYMLINK_MAX) {
			return Err(errno!(ENAMETOOLONG));
		}
		let fs = downcast_fs::<Ext2Fs>(&*node.fs.ops);
		let mut inode_ = Ext2INode::get(node, fs)?;
		if inode_.get_type() != FileType::Link {
			return Err(errno!(EINVAL));
		}
		// Get storage slice
		let inline = buf.len() <= inode::SYMLINK_INLINE_LIMIT as usize;
		if inline {
			// Store inline
			let dst = bytes::as_bytes_mut(&mut inode_.i_block);
			dst[..buf.len()].copy_from_slice(buf);
			dst[buf.len()..].fill(0);
		} else {
			// Allocate a block
			let blk_off = inode_.alloc_content_blk(0, fs)?;
			inode_.i_block[0] = blk_off;
			let blk = read_block(fs, blk_off as _)?;
			// No one else can access the block since we just allocated it
			let dst = unsafe { blk.slice_mut() };
			// Copy
			dst[..buf.len()].copy_from_slice(buf);
			dst[buf.len()..].fill(0);
		}
		// Update size
		inode_.set_size(&fs.sp, buf.len() as _, inline);
		node.stat.lock().size = buf.len() as _;
		inode_.mark_dirty();
		Ok(())
	}

	fn rename(&self, entry: &vfs::Entry, new_parent: &vfs::Entry, new_name: &[u8]) -> EResult<()> {
		let entry_node = entry.node();
		let fs = downcast_fs::<Ext2Fs>(&*entry_node.fs.ops);
		if unlikely(fs.readonly) {
			return Err(errno!(EROFS));
		}
		// Create new entry
		let dir = {
			let new_parent_node = new_parent.node();
			let mut new_parent_inode = Ext2INode::get(new_parent_node, fs)?;
			// Check the entry does not exist
			if new_parent_inode.get_dirent(new_name, fs)?.is_some() {
				return Err(errno!(EEXIST));
			}
			let mut inode = Ext2INode::get(entry.node(), fs)?;
			let dir = inode.get_type() == FileType::Directory;
			// Update the `..` entry
			if dir {
				if unlikely(new_parent_inode.i_links_count == u16::MAX) {
					return Err(errno!(EMFILE));
				}
				let (_, off) = inode
					.get_dirent(b"..", fs)?
					.ok_or_else(|| errno!(EUCLEAN))?;
				inode.set_dirent_inode(off, new_parent_node.inode, fs)?;
				// Update links count
				new_parent_inode.i_links_count += 1;
				new_parent.node().stat.lock().nlink = new_parent_inode.i_links_count;
			}
			new_parent_inode.add_dirent(fs, entry_node.inode as _, new_name, inode.get_type())?;
			new_parent_inode.mark_dirty();
			inode.mark_dirty();
			dir
		};
		// Remove old entry
		let old_parent = entry.parent.as_ref().unwrap();
		let old_parent_node = old_parent.node();
		let mut old_parent_inode = Ext2INode::get(old_parent_node, fs)?;
		let (_, off) = old_parent_inode
			.get_dirent(&entry.name, fs)?
			.ok_or_else(|| errno!(ENOENT))?;
		old_parent_inode.set_dirent_inode(off, 0, fs)?;
		// Update links count
		if dir {
			old_parent_inode.i_links_count = old_parent_inode.i_links_count.saturating_sub(1);
			old_parent_node.stat.lock().nlink = old_parent_inode.i_links_count;
		}
		old_parent_inode.mark_dirty();
		Ok(())
	}

	fn read_page(&self, node: &Arc<Node>, off: u64) -> EResult<RcFrame> {
		node.mapped.get_or_insert_frame(off, 0, || {
			let fs = downcast_fs::<Ext2Fs>(&*node.fs.ops);
			let inode = Ext2INode::get(node, fs)?;
			let off: u32 = off.try_into().map_err(|_| errno!(EOVERFLOW))?;
			let blk_off = inode
				.translate_blk_off(off, fs)?
				.ok_or_else(|| errno!(EOVERFLOW))?;
			fs.dev
				.ops
				.read_frame(blk_off.get() as _, 0, FrameOwner::Node(node.clone()))
		})
	}

	fn write_frame(&self, node: &Node, frame: &RcFrame) -> EResult<()> {
		let fs = downcast_fs::<Ext2Fs>(&*node.fs.ops);
		fs.dev.ops.write_pages(frame.dev_offset(), frame.slice())
	}

	fn set_stat(&self, node: &Node, stat: &Stat) -> EResult<()> {
		let fs = downcast_fs::<Ext2Fs>(&*node.fs.ops);
		let mut inode_ = Ext2INode::get(node, fs)?;
		inode_.set_permissions(stat.mode);
		inode_.i_uid = stat.uid;
		inode_.i_gid = stat.gid;
		inode_.i_ctime = stat.ctime as _;
		inode_.i_mtime = stat.mtime as _;
		inode_.i_atime = stat.atime as _;
		inode_.mark_dirty();
		Ok(())
	}
}

/// Open file operations.
#[derive(Debug)]
pub struct Ext2FileOps;

impl FileOps for Ext2FileOps {
	fn read(&self, file: &File, off: u64, buf: UserSlice<u8>) -> EResult<usize> {
		// TODO replace by filetype-specific FileOps
		let node = file.node();
		let fs = downcast_fs::<Ext2Fs>(&*node.fs.ops);
		{
			let inode_ = Ext2INode::get(node, fs)?;
			if inode_.get_type() != FileType::Regular {
				return Err(errno!(EINVAL));
			}
		}
		// TODO O_DIRECT
		generic_file_read(file, off, buf)
	}

	fn write(&self, file: &File, off: u64, buf: UserSlice<u8>) -> EResult<usize> {
		let node = file.node();
		let fs = downcast_fs::<Ext2Fs>(&*node.fs.ops);
		if unlikely(fs.readonly) {
			return Err(errno!(EROFS));
		}
		// TODO replace by filetype-specific FileOps
		{
			let inode_ = Ext2INode::get(node, fs)?;
			if inode_.get_type() != FileType::Regular {
				return Err(errno!(EINVAL));
			}
		}
		// TODO O_DIRECT
		generic_file_write(file, off, buf)
	}

	fn truncate(&self, file: &File, size: u64) -> EResult<()> {
		let node = file.node();
		let fs = downcast_fs::<Ext2Fs>(&*node.fs.ops);
		if unlikely(fs.readonly) {
			return Err(errno!(EROFS));
		}
		let mut inode_ = Ext2INode::get(node, fs)?;
		// TODO replace by filetype-specific FileOps
		if inode_.get_type() != FileType::Regular {
			return Err(errno!(EINVAL));
		}
		// The size of a block
		let blk_size = fs.sp.get_block_size();
		let old_size = inode_.get_size(&fs.sp);
		if size < old_size {
			// Shrink the file
			let start = size.div_ceil(blk_size as _) as u32;
			let end = old_size.div_ceil(blk_size as _) as u32;
			for off in start..end {
				inode_.free_content_blk(off, fs)?;
			}
			// Clear cache
			node.mapped.truncate(start as _);
		} else {
			// Expand the file
			let start = old_size.div_ceil(blk_size as _) as u32;
			let end = size.div_ceil(blk_size as _) as u32;
			for off in start..end {
				inode_.alloc_content_blk(off, fs)?;
			}
		}
		// Update size
		inode_.set_size(&fs.sp, size, false);
		inode_.mark_dirty();
		node.stat.lock().size = size;
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
	fn read(dev: &Arc<BlkDev>) -> EResult<RcFrameVal<Self>> {
		let page = BlkDev::read_frame(dev, 0, 0, FrameOwner::BlkDev(dev.clone()))?;
		Ok(RcFrameVal::new(page, 1))
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
			max(self.s_first_ino, ROOT_DIRECTORY_INODE + 1)
		} else {
			10
		}
	}
}

/// An instance of the ext2 filesystem.
#[derive(Debug)]
struct Ext2Fs {
	/// The device on which the filesystem is located
	dev: Arc<BlkDev>,
	/// The filesystem's superblock
	sp: RcFrameVal<Superblock>,
	/// Tells whether the filesystem is mounted as read-only
	readonly: bool,
}

impl Ext2Fs {
	/// Finds a free element in the given bitmap, allocates it, and returns its index.
	///
	/// Arguments:
	/// - `start` is the starting block to search into
	/// - `size` is the number of elements in the bitmap
	fn bitmap_alloc(&self, start_blk: u32, size: u32) -> EResult<Option<u32>> {
		let blk_size = self.sp.get_block_size();
		let end_blk = start_blk + size.div_ceil(blk_size * 8);
		// Iterate on blocks
		for blk_off in start_blk..end_blk {
			let blk = read_block(self, blk_off as _)?;
			if let Some(off) = bitmap_alloc_impl(&blk) {
				let blk_off = blk_off - start_blk;
				return Ok(Some(blk_off * blk_size * 8 + off));
			}
		}
		Ok(None)
	}

	/// Frees the element at `index` in the bitmap starting at the block `start_blk`.
	///
	/// The function returns the previous value of the bit.
	fn bitmap_free(&self, start_blk: u32, index: u32) -> EResult<bool> {
		// Get block
		let blk_size = self.sp.get_block_size();
		let blk_off = start_blk + index / (blk_size * 8);
		let blk = read_block(self, blk_off as _)?;
		// Atomically clear bit
		let bitmap_byte_index = index / 8;
		let byte = &blk.slice::<AtomicU8>()[bitmap_byte_index as usize];
		let bitmap_bit_index = index % 8;
		// Atomic write and mark as dirty
		let prev = byte.fetch_and(!(1 << bitmap_bit_index), Release);
		blk.mark_page_dirty(bitmap_byte_index as usize / PAGE_SIZE);
		Ok(prev & (1 << bitmap_bit_index) != 0)
	}

	/// Allocates an inode and returns its ID.
	///
	/// `directory` tells whether the inode is allocated for a directory.
	///
	/// If no free inode can be found, the function returns an error.
	pub fn alloc_inode(&self, directory: bool) -> EResult<u32> {
		if unlikely(self.sp.s_free_inodes_count.load(Acquire) == 0) {
			return Err(errno!(ENOSPC));
		}
		for group in 0..self.sp.get_block_groups_count() {
			let bgd = BlockGroupDescriptor::get(group, self)?;
			if bgd.bg_free_inodes_count.load(Acquire) == 0 {
				continue;
			}
			if let Some(j) = self.bitmap_alloc(bgd.bg_inode_bitmap, self.sp.s_inodes_per_group)? {
				self.sp.s_free_inodes_count.fetch_sub(1, Release);
				bgd.bg_free_inodes_count.fetch_sub(1, Release);
				if directory {
					bgd.bg_used_dirs_count.fetch_add(1, Release);
				}
				self.sp.mark_dirty();
				bgd.mark_dirty();
				return Ok(group * self.sp.s_inodes_per_group + j + 1);
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
	pub fn free_inode(&self, inode: INode, directory: bool) -> EResult<()> {
		// Validation
		let inode: u32 = inode.try_into().map_err(|_| errno!(EOVERFLOW))?;
		if unlikely(inode == 0) {
			return Ok(());
		}
		// Get block group
		let group = (inode - 1) / self.sp.s_inodes_per_group;
		let bgd = BlockGroupDescriptor::get(group, self)?;
		// Clear bit and update counters
		let bitfield_index = (inode - 1) % self.sp.s_inodes_per_group;
		let prev = self.bitmap_free(bgd.bg_inode_bitmap, bitfield_index)?;
		// Check to avoid overflow in case of corrupted filesystem
		if prev {
			self.sp.s_free_inodes_count.fetch_add(1, Release);
			bgd.bg_free_inodes_count.fetch_add(1, Release);
			if directory {
				bgd.bg_used_dirs_count.fetch_sub(1, Release);
			}
			self.sp.mark_dirty();
			bgd.mark_dirty();
		}
		Ok(())
	}

	/// Returns the ID of a free block in the filesystem.
	pub fn alloc_block(&self) -> EResult<u32> {
		if unlikely(self.sp.s_free_inodes_count.load(Acquire) == 0) {
			return Err(errno!(ENOSPC));
		}
		for i in 0..self.sp.get_block_groups_count() {
			let bgd = BlockGroupDescriptor::get(i as _, self)?;
			if bgd.bg_free_blocks_count.load(Acquire) == 0 {
				continue;
			}
			let Some(j) = self.bitmap_alloc(bgd.bg_block_bitmap, self.sp.s_blocks_per_group)?
			else {
				continue;
			};
			let blk_index = i * self.sp.s_blocks_per_group + j;
			if unlikely(blk_index <= 2 || blk_index >= self.sp.s_blocks_count) {
				return Err(errno!(EUCLEAN));
			}
			self.sp.s_free_blocks_count.fetch_sub(1, Release);
			bgd.bg_free_blocks_count.fetch_sub(1, Release);
			self.sp.mark_dirty();
			bgd.mark_dirty();
			return Ok(blk_index);
		}
		Err(errno!(ENOSPC))
	}

	/// Marks the block `blk` available on the filesystem.
	///
	/// If `blk` is zero, the function does nothing.
	pub fn free_block(&self, blk: u32) -> EResult<()> {
		// Validation
		if unlikely(blk <= 2 || blk >= self.sp.s_blocks_count) {
			return Err(errno!(EUCLEAN));
		}
		// Get block group
		let group = blk / self.sp.s_blocks_per_group;
		let bgd = BlockGroupDescriptor::get(group, self)?;
		// Clear bit and update counters
		let bitfield_index = blk % self.sp.s_blocks_per_group;
		let prev = self.bitmap_free(bgd.bg_block_bitmap, bitfield_index)?;
		// Check to avoid overflow in case of corrupted filesystem
		if prev {
			self.sp.s_free_blocks_count.fetch_add(1, Release);
			bgd.bg_free_blocks_count.fetch_add(1, Release);
			self.sp.mark_dirty();
			bgd.mark_dirty();
		}
		Ok(())
	}
}

// TODO Update the write timestamp when the fs is written (take mount flags into
// account)
impl FilesystemOps for Ext2Fs {
	fn get_name(&self) -> &[u8] {
		b"ext2"
	}

	fn cache_entries(&self) -> bool {
		true
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
			f_namelen: NAME_MAX as _,
			f_frsize: math::pow2(self.sp.s_log_frag_size + 10),
			f_flags: 0, // TODO
		})
	}

	fn root(&self, fs: &Arc<Filesystem>) -> EResult<Arc<Node>> {
		fs.node_get_or_insert(ROOT_DIRECTORY_INODE as _, || {
			let mut node = Node::new(
				ROOT_DIRECTORY_INODE as _,
				fs.clone(),
				Default::default(),
				Box::new(Ext2NodeOps)?,
				Box::new(Ext2FileOps)?,
			);
			let stat = Ext2INode::get(&node, self)?.stat(&self.sp);
			node.stat = Spin::new(stat);
			Ok(Arc::new(node)?)
		})
	}

	fn create_node(&self, fs: &Arc<Filesystem>, stat: Stat) -> EResult<Arc<Node>> {
		if unlikely(self.readonly) {
			return Err(errno!(EROFS));
		}
		let file_type = stat.get_type().ok_or_else(|| errno!(EINVAL))?;
		// Allocate an inode
		let inode_index = self.alloc_inode(file_type == FileType::Directory)?;
		// Create inode
		let mut node = Node::new(
			inode_index as _,
			fs.clone(),
			Default::default(),
			Box::new(Ext2NodeOps)?,
			Box::new(Ext2FileOps)?,
		);
		let mut inode = Ext2INode::get(&node, self)?;
		*inode = Ext2INode {
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
				inode.add_dirent(self, inode_index, b".", FileType::Directory)?;
				inode.i_links_count += 1;
			}
			FileType::BlockDevice | FileType::CharDevice => {
				inode.set_device(stat.dev_major as u8, stat.dev_minor as u8);
			}
			_ => {}
		}
		inode.mark_dirty();
		// Update stat on `node` and return it
		let stat = inode.stat(&self.sp);
		drop(inode);
		node.stat = Spin::new(stat);
		// Insert in cache
		let node = Arc::new(node)?;
		fs.node_insert(node.clone())?;
		Ok(node)
	}

	fn destroy_node(&self, node: &Node) -> EResult<()> {
		if unlikely(self.readonly) {
			return Err(errno!(EROFS));
		}
		let mut inode = Ext2INode::get(node, self)?;
		// Remove the inode
		inode.i_links_count = 0;
		let ts = current_time_sec(Clock::Monotonic);
		inode.i_dtime = ts as _;
		inode.free_content(self)?;
		inode.mark_dirty();
		// Free inode
		self.free_inode(node.inode, inode.get_type() == FileType::Directory)?;
		Ok(())
	}

	fn sync_fs(&self) -> EResult<()> {
		self.dev.mapped.sync()
	}
}

/// The ext2 filesystem type.
pub struct Ext2FsType;

impl FilesystemType for Ext2FsType {
	fn get_name(&self) -> &'static [u8] {
		b"ext2"
	}

	fn detect(&self, dev: &Arc<BlkDev>) -> EResult<bool> {
		Superblock::read(dev).map(|sp| sp.is_valid())
	}

	fn load_filesystem(
		&self,
		dev: Option<Arc<BlkDev>>,
		_mountpath: PathBuf,
		readonly: bool,
	) -> EResult<Arc<Filesystem>> {
		let dev = dev.ok_or_else(|| errno!(ENODEV))?;
		let sp = Superblock::read(&dev)?;
		if unlikely(!sp.is_valid()) {
			return Err(errno!(EINVAL));
		}
		if unlikely(sp.s_log_block_size < 2) {
			return Err(errno!(EINVAL));
		}
		if sp.s_rev_level >= 1 {
			if unlikely(
				!sp.s_inode_size.is_power_of_two()
					|| sp.s_inode_size < 128
					|| sp.s_inode_size as u32 > sp.get_block_size(),
			) {
				return Err(errno!(EINVAL));
			}
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
		let ts = current_time_sec(Clock::Monotonic);
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
		sp.s_mtime.store(ts as _, Relaxed);
		sp.s_mnt_count.fetch_add(1, Relaxed);
		sp.mark_dirty();
		Ok(Filesystem::new(
			dev.id.get_device_number(),
			Box::new(Ext2Fs {
				dev,
				sp,
				readonly,
			})?,
		)?)
	}
}
