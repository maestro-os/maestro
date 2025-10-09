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

//! An inode represents a file in the filesystem.

use super::{
	Ext2Fs, Superblock, bgd::BlockGroupDescriptor, dirent, dirent::Dirent, read_block, zero_block,
};
use crate::{
	file::{FileType, INode, Mode, Stat, fs::ext2::dirent::DirentIterator, vfs::node::Node},
	memory::cache::{RcFrame, RcFrameVal},
	sync::spin::SpinGuard,
};
use core::{
	hint::unlikely,
	mem,
	num::NonZeroU32,
	ops::{Deref, DerefMut},
	sync::atomic::{AtomicU32, Ordering::Relaxed},
};
use macros::AnyRepr;
use utils::{
	errno,
	errno::EResult,
	limits::{NAME_MAX, PAGE_SIZE},
	math,
};

/// The maximum number of direct blocks for each inodes.
pub const DIRECT_BLOCKS_COUNT: usize = 12;

/// INode type: FIFO
pub const INODE_TYPE_FIFO: u16 = 0x1000;
/// INode type: Char device
pub const INODE_TYPE_CHAR_DEVICE: u16 = 0x2000;
/// INode type: Directory
pub const INODE_TYPE_DIRECTORY: u16 = 0x4000;
/// INode type: Block device
pub const INODE_TYPE_BLOCK_DEVICE: u16 = 0x6000;
/// INode type: Regular file
pub const INODE_TYPE_REGULAR: u16 = 0x8000;
/// INode type: Symbolic link
pub const INODE_TYPE_SYMLINK: u16 = 0xa000;
/// INode type: Socket
pub const INODE_TYPE_SOCKET: u16 = 0xc000;

/// `s_flags`: Secure deletion
const INODE_FLAG_SECURE_DELETION: u32 = 0x00001;
/// `s_flags`: Keep a copy of data when deleted
const INODE_FLAG_DELETE_COPY: u32 = 0x00002;
/// `s_flags`: File compression
const INODE_FLAG_COMPRESSION: u32 = 0x00004;
/// `s_flags`: Synchronous updates
const INODE_FLAG_SYNC: u32 = 0x00008;
/// `s_flags`: Immutable file
const INODE_FLAG_IMMUTABLE: u32 = 0x00010;
/// `s_flags`: Append only
const INODE_FLAG_APPEND_ONLY: u32 = 0x00020;
/// `s_flags`: File is not included in 'dump' command
const INODE_FLAG_NODUMP: u32 = 0x00040;
/// `s_flags`: Last accessed time should not be updated
const INODE_FLAG_ATIME_NOUPDATE: u32 = 0x00080;
/// `s_flags`: Hash indexed directory
const INODE_FLAG_HASH_INDEXED: u32 = 0x10000;
/// `s_flags`: AFS directory
const INODE_FLAG_AFS_DIRECTORY: u32 = 0x20000;
/// `s_flags`: Journal file data
const INODE_FLAG_JOURNAL_FILE: u32 = 0x40000;

/// The size of a sector in bytes.
const SECTOR_SIZE: u32 = 512;

/// The maximum length for a symlink to be stored in the inode itself instead of a
/// separate block.
pub const SYMLINK_INLINE_LIMIT: u64 = 60;

/// The inode of the root directory.
pub const ROOT_DIRECTORY_INODE: u32 = 2;

/// Container for an inode, locking its associated spinlock to avoid concurrency issues
pub(super) struct INodeWrap<'n> {
	_guard: SpinGuard<'n, (), true>,
	inode: RcFrameVal<Ext2INode>,
}

impl INodeWrap<'_> {
	/// Marks the associated page as dirty.
	#[inline]
	pub fn mark_dirty(&self) {
		self.inode.mark_dirty()
	}
}

impl Deref for INodeWrap<'_> {
	type Target = Ext2INode;

	fn deref(&self) -> &Self::Target {
		self.inode.deref()
	}
}

impl DerefMut for INodeWrap<'_> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		unsafe { self.inode.as_mut() }
	}
}

/// Computes the indirection offsets to reach the block at the linear offset `off`.
///
/// Arguments:
/// - `ent_per_blk_log` is the log2 of the number of entries in a block.
/// - `offsets` is the array to which the offsets are written.
///
/// On success, the function returns the number of offsets.
///
/// If the offset is out of bounds, the function returns [`EOVERFLOW`].
fn indirections_offsets(
	mut off: u32,
	ent_per_blk_log: u32,
	offsets: &mut [usize; 4],
) -> EResult<usize> {
	offsets.fill(0);
	if off < DIRECT_BLOCKS_COUNT as u32 {
		offsets[0] = off as _;
		return Ok(1);
	}
	off -= DIRECT_BLOCKS_COUNT as u32;
	let ent_per_blk = math::pow2(ent_per_blk_log as _);
	if off < ent_per_blk {
		offsets[0] = DIRECT_BLOCKS_COUNT;
		offsets[1] = off as _;
		return Ok(2);
	}
	off -= ent_per_blk;
	if off < ent_per_blk * ent_per_blk {
		offsets[0] = DIRECT_BLOCKS_COUNT + 1;
		offsets[1] = (off >> ent_per_blk_log) as _;
		offsets[2] = (off & (ent_per_blk - 1)) as _;
		return Ok(3);
	}
	off -= ent_per_blk * ent_per_blk;
	if off < ent_per_blk * ent_per_blk * ent_per_blk {
		offsets[0] = DIRECT_BLOCKS_COUNT + 2;
		offsets[1] = (off >> (ent_per_blk_log * 2)) as _;
		offsets[2] = ((off >> ent_per_blk_log) & (ent_per_blk_log - 1)) as _;
		offsets[3] = (off & (ent_per_blk - 1)) as _;
		return Ok(4);
	}
	Err(errno!(EOVERFLOW))
}

/// Checks for an invalid block number.
///
/// If the block number is zero, the function returns `None`.
pub fn check_blk_off(blk: u32, sp: &Superblock) -> EResult<Option<NonZeroU32>> {
	if unlikely(blk >= sp.s_blocks_count) {
		return Err(errno!(EUCLEAN));
	}
	Ok(NonZeroU32::new(blk))
}

/// Tells whether the block contains only free directory entries.
fn is_block_empty(blk: &mut [u8], sp: &Superblock) -> EResult<bool> {
	let mut off = 0;
	while off < blk.len() {
		let ent = Dirent::from_slice(&mut blk[off..], sp)?;
		if !ent.is_free() {
			return Ok(false);
		}
		off += ent.rec_len as usize;
	}
	Ok(true)
}

/// Fills the given slice with empty directory entries.
///
/// It is the caller's responsibility to ensure the starting offset is properly aligned to
/// [`dirent::ALIGN`].
///
/// If an entry could not be created, the associated error is returned.
fn fill_free_entries(buf: &mut [u8], sp: &Superblock) -> EResult<()> {
	const MIN: usize = dirent::NAME_OFF;
	const MAX: usize = u16::MAX as usize;
	const SPECIAL_CASE_END: usize = MAX + MIN;
	let mut i = 0;
	loop {
		let rec_len = match buf.len() - i {
			// Special case: a max-sized entry would leave a space too small to be filled
			MAX..SPECIAL_CASE_END => (MAX / 2).next_multiple_of(dirent::ALIGN),
			// Clamp to maximum size
			SPECIAL_CASE_END.. => MAX,
			// An entry could not fill the remaining space: stop
			//
			// This can only happen when reaching the end, unless the starting offset is
			// misaligned, which is invalid
			..MIN => break,
			// Fill the remaining space
			r => r,
		};
		Dirent::write_new(&mut buf[i..], sp, 0, rec_len as _, None, b"")?;
		i += rec_len;
	}
	Ok(())
}

/// An inode represents a file in the filesystem.
///
/// The name of the file is not included in the inode but in the directory entry associated with it
/// since several entries can refer to the same inode (hard links).
#[repr(C)]
#[derive(AnyRepr, Clone)]
pub struct Ext2INode {
	/// Type and permissions.
	pub i_mode: u16,
	/// User ID.
	pub i_uid: u16,
	/// Lower 32 bits of size in bytes.
	pub i_size: u32,
	/// Timestamp of the last modification of the metadata.
	pub i_ctime: u32,
	/// Timestamp of the last modification of the content.
	pub i_mtime: u32,
	/// Timestamp of the last access.
	pub i_atime: u32,
	/// Timestamp of the deletion.
	pub i_dtime: u32,
	/// Group ID.
	pub i_gid: u16,
	/// The number of hard links to this inode.
	pub i_links_count: u16,
	/// The number of sectors used by this inode.
	pub i_blocks: u32,
	/// INode flags.
	pub i_flags: u32,
	/// OS-specific value.
	pub i_osd1: u32,
	/// Direct block pointers.
	pub i_block: [u32; DIRECT_BLOCKS_COUNT + 3],
	/// Generation number.
	pub i_generation: u32,
	/// The file's ACL.
	pub i_file_acl: u32,
	/// Higher 32 bits of size in bytes.
	pub i_dir_acl: u32,
	/// Block address of fragment.
	pub i_faddr: u32,
	/// OS-specific value.
	pub i_osd2: [u8; 12],
}

impl Ext2INode {
	/// Returns the `i`th inode on the filesystem.
	pub fn get<'n>(node: &'n Node, fs: &Ext2Fs) -> EResult<INodeWrap<'n>> {
		let i: u32 = node.inode.try_into().map_err(|_| errno!(EOVERFLOW))?;
		// Check the index is correct
		let Some(i) = i.checked_sub(1) else {
			return Err(errno!(EINVAL));
		};
		let blk_size = fs.sp.get_block_size() as u64;
		let inode_size = fs.sp.get_inode_size() as u64;
		// Read BGD
		let blk_grp = i / fs.sp.s_inodes_per_group;
		let bgd = BlockGroupDescriptor::get(blk_grp, fs)?;
		let inode_grp_off = i % fs.sp.s_inodes_per_group;
		let inode_table_blk_off = (inode_grp_off as u64 * inode_size) / blk_size;
		// Read the block containing the inode
		let blk_off = bgd.bg_inode_table as u64 + inode_table_blk_off;
		let blk = read_block(fs, blk_off)?;
		// Entry offset
		let off = i as u64 % (blk_size / inode_size);
		// Adapt to the size of an inode
		let off = off * (inode_size / 128);
		Ok(INodeWrap {
			_guard: node.lock.lock(),
			inode: RcFrameVal::new(blk, off as _),
		})
	}

	/// Returns the file's status.
	pub fn stat(&self, sp: &Superblock) -> Stat {
		let (dev_major, dev_minor) = self.get_device();
		Stat {
			mode: self.i_mode as _,
			nlink: self.i_links_count as _,
			uid: self.i_uid,
			gid: self.i_gid,
			size: self.get_size(sp),
			blocks: self.i_blocks as _,
			dev_major: dev_major as _,
			dev_minor: dev_minor as _,
			ctime: self.i_ctime as _,
			mtime: self.i_mtime as _,
			atime: self.i_atime as _,
		}
	}

	/// Returns the type of the file.
	pub fn get_type(&self) -> FileType {
		let file_type = self.i_mode & 0xf000;
		match file_type {
			INODE_TYPE_FIFO => FileType::Fifo,
			INODE_TYPE_CHAR_DEVICE => FileType::CharDevice,
			INODE_TYPE_DIRECTORY => FileType::Directory,
			INODE_TYPE_BLOCK_DEVICE => FileType::BlockDevice,
			INODE_TYPE_REGULAR => FileType::Regular,
			INODE_TYPE_SYMLINK => FileType::Link,
			INODE_TYPE_SOCKET => FileType::Socket,
			_ => FileType::Regular,
		}
	}

	/// Sets the permissions of the file.
	pub fn set_permissions(&mut self, perm: Mode) {
		self.i_mode = (self.i_mode & !0o7777) | (perm & 0o7777) as u16;
	}

	/// Returns the size of the file.
	///
	/// `superblock` is the filesystem's superblock.
	pub fn get_size(&self, sp: &Superblock) -> u64 {
		let has_version = sp.s_rev_level >= 1;
		let has_feature = sp.s_feature_ro_compat & super::WRITE_REQUIRED_64_BITS != 0;
		if has_version && has_feature {
			((self.i_dir_acl as u64) << 32) | (self.i_size as u64)
		} else {
			self.i_size as u64
		}
	}

	/// Sets the file's size.
	///
	/// Arguments:
	/// - `superblock` is the filesystem's superblock
	/// - `size` is the file's size
	/// - `inline` is `true` if the inode is a symlink storing the target inline
	pub fn set_size(&mut self, sp: &Superblock, size: u64, inline: bool) {
		let has_version = sp.s_rev_level >= 1;
		let has_feature = sp.s_feature_ro_compat & super::WRITE_REQUIRED_64_BITS != 0;
		if has_version && has_feature {
			self.i_dir_acl = (size >> 32) as u32;
		}
		self.i_size = size as u32;
		if !inline {
			let blk_size = sp.get_block_size();
			let sector_per_blk = blk_size / SECTOR_SIZE;
			self.i_blocks = size.div_ceil(blk_size as _) as u32 * sector_per_blk;
		} else {
			self.i_blocks = 0;
		}
	}

	/// Returns the number of content blocks.
	pub fn get_blocks(&self, sp: &Superblock) -> u32 {
		let sector_per_blk = sp.get_block_size() / SECTOR_SIZE;
		self.i_blocks.div_ceil(sector_per_blk)
	}

	/// Translates the given file block offset `off` to disk block offset.
	///
	/// If the block does not exist, the function returns `None`.
	pub fn translate_blk_off(&self, off: u32, fs: &Ext2Fs) -> EResult<Option<NonZeroU32>> {
		let mut offsets: [usize; 4] = [0; 4];
		let depth = indirections_offsets(off, fs.sp.get_entries_per_block_log(), &mut offsets)?;
		let Some(mut blk_off) = check_blk_off(self.i_block[offsets[0]], &fs.sp)? else {
			return Ok(None);
		};
		// Perform indirections
		for off in &offsets[1..depth] {
			let blk = read_block(fs, blk_off.get() as _)?;
			let Some(b) = check_blk_off(blk.slice()[*off], &fs.sp)? else {
				return Ok(None);
			};
			blk_off = b;
		}
		Ok(Some(blk_off))
	}

	/// Allocates a block for the node's content block at the given file block offset `off`.
	///
	/// The content of the allocated block is **not** initialized.
	///
	/// If a block is already allocated, the function does nothing.
	///
	/// **Note**: the function assumes the inode is locked.
	///
	/// On success, the function returns the allocated disk block offset.
	pub fn alloc_content_blk(&mut self, off: u32, fs: &Ext2Fs) -> EResult<u32> {
		let mut offsets: [usize; 4] = [0; 4];
		let depth = indirections_offsets(off, fs.sp.get_entries_per_block_log(), &mut offsets)?;
		// Allocate the first level if needed
		let blk_off = &mut self.i_block[offsets[0]];
		if *blk_off == 0 {
			*blk_off = fs.alloc_block()?;
			zero_block(fs, *blk_off as _)?;
		}
		// Perform indirections
		let mut blk_off = *blk_off;
		for off in &offsets[1..depth] {
			let blk = read_block(fs, blk_off as _)?;
			let ent = &blk.slice::<AtomicU32>()[*off];
			// Allocate block if needed (two atomic operations are fine here since the node is
			// locked)
			let mut b = ent.load(Relaxed);
			if b == 0 {
				let new = fs.alloc_block()?;
				zero_block(fs, new as _)?;
				ent.store(new, Relaxed);
				blk.mark_page_dirty(*off / (PAGE_SIZE / size_of::<AtomicU32>()));
				b = new;
			}
			blk_off = b;
		}
		Ok(blk_off)
	}

	fn free_content_blk_impl(blk: u32, offsets: &[usize], fs: &Ext2Fs) -> EResult<bool> {
		let Some(off) = offsets.first() else {
			return Ok(true);
		};
		let blk = read_block(fs, blk as _)?;
		let ents = blk.slice::<AtomicU32>();
		let ent = &ents[*off];
		// Handle child block and determine whether the entry in the current block should be freed
		let free = Self::free_content_blk_impl(ent.load(Relaxed), &offsets[1..], fs)?;
		if free {
			let b = ent.swap(0, Relaxed);
			blk.mark_page_dirty(*off / (PAGE_SIZE / size_of::<AtomicU32>()));
			let empty = ents.iter().all(|b| b.load(Relaxed) == 0);
			fs.free_block(b)?;
			Ok(empty)
		} else {
			Ok(false)
		}
	}

	/// Frees a content block at the given file block offset `off`.
	///
	/// If the block is not allocated, the function does nothing.
	pub fn free_content_blk(&mut self, off: u32, fs: &Ext2Fs) -> EResult<()> {
		let mut offsets: [usize; 4] = [0; 4];
		let depth = indirections_offsets(off, fs.sp.get_entries_per_block_log(), &mut offsets)?;
		let blk = &mut self.i_block[offsets[0]];
		if check_blk_off(*blk, &fs.sp)?.is_none() {
			return Ok(());
		}
		if Self::free_content_blk_impl(*blk, &offsets[1..depth], fs)? {
			let blk = mem::take(blk);
			fs.free_block(blk)?;
		}
		Ok(())
	}

	/// Frees all content blocks by doing redirections.
	///
	/// `level` is the number of indirections
	fn indirect_free_all(blk_off: u32, level: usize, fs: &Ext2Fs) -> EResult<()> {
		let blk = read_block(fs, blk_off as _)?;
		for blk in blk.slice() {
			let Some(blk) = check_blk_off(*blk, &fs.sp)? else {
				continue;
			};
			if let Some(next_level) = level.checked_sub(1) {
				Self::indirect_free_all(blk.get(), next_level, fs)?;
			}
			fs.free_block(blk.get())?;
		}
		Ok(())
	}

	/// Frees all the content blocks of the inode.
	pub fn free_content(&mut self, fs: &Ext2Fs) -> EResult<()> {
		// If the file is a link and its content is stored inline, there is nothing to do
		if matches!(self.get_type(), FileType::Link)
			&& self.get_size(&fs.sp) <= SYMLINK_INLINE_LIMIT
		{
			return Ok(());
		}
		self.set_size(&fs.sp, 0, false);
		// Free blocks
		for (off, blk) in self.i_block.iter().enumerate() {
			let Some(blk) = check_blk_off(*blk, &fs.sp)? else {
				continue;
			};
			let depth = off.saturating_sub(DIRECT_BLOCKS_COUNT);
			if let Some(depth) = depth.checked_sub(1) {
				Self::indirect_free_all(blk.get(), depth, fs)?;
			}
			fs.free_block(blk.get())?;
		}
		self.i_block.fill(0);
		Ok(())
	}

	/// Returns the information of a directory entry with the given name `name`.
	///
	/// The function returns:
	/// - The inode
	/// - The offset of the entry
	///
	/// If the entry doesn't exist, the function returns `None`.
	///
	/// If the file is not a directory, the function returns `None`.
	pub fn get_dirent(&self, name: &[u8], fs: &Ext2Fs) -> EResult<Option<(u32, u64)>> {
		// Validation
		if self.get_type() != FileType::Directory {
			return Ok(None);
		}
		// TODO If the hash index is enabled, use it
		// Linear lookup
		let mut blk = None;
		for ent in DirentIterator::new(fs, self, &mut blk, 0)? {
			let (off, ent) = ent?;
			if !ent.is_free() && ent.get_name(&fs.sp) == name {
				return Ok(Some((ent.inode, off)));
			}
		}
		Ok(None)
	}

	/// Tells whether the current directory is empty.
	pub fn is_directory_empty(&self, fs: &Ext2Fs) -> EResult<bool> {
		let mut blk = None;
		for ent in DirentIterator::new(fs, self, &mut blk, 0)? {
			let (_, ent) = ent?;
			if !ent.is_free() {
				let name = ent.get_name(&fs.sp);
				if name != b"." && name != b".." {
					return Ok(false);
				}
			}
		}
		Ok(true)
	}

	/// Looks for a sequence of free entries large enough to fit a chunk with at least `min_size`
	/// bytes.
	///
	/// Return values:
	/// - The block containing the sequence
	/// - The offset to the beginning of the sequence
	/// - The length of the sequence
	///
	/// Arguments:
	/// - `buf` is the block buffer
	/// - `min_size` is the minimum size of the new entry in bytes
	///
	/// If no suitable sequence is found, the function returns `None`.
	fn find_suitable_slot(
		&self,
		fs: &Ext2Fs,
		min_size: u16,
	) -> EResult<Option<(RcFrame, u64, usize)>> {
		let blk_size = fs.sp.get_block_size() as u64;
		let mut begin = 0;
		let mut free_length = 0;
		let mut blk = None;
		for ent in DirentIterator::new(fs, self, &mut blk, 0)? {
			let (off, ent) = ent?;
			let end_off = off + ent.rec_len as u64;
			if ent.is_free() {
				free_length += ent.rec_len as usize;
			}
			// If used entry, or at the end of the block
			let end = off / blk_size != end_off / blk_size;
			if !ent.is_free() || end {
				// If a sequence large enough has been found, stop
				if free_length >= min_size as usize {
					return Ok(Some((blk.unwrap(), begin, free_length)));
				}
				// Reset counter
				free_length = 0;
				begin = end_off;
			}
		}
		Ok(None)
	}

	/// Adds a new entry to the current directory.
	///
	/// Arguments:
	/// - `entry_inode` is the inode of the entry
	/// - `name` is the name of the entry
	/// - `file_type` is the type of the entry
	///
	/// If the block allocation fails or if the entry name is already used, the
	/// function returns an error.
	///
	/// If the file is not a directory, the behaviour is undefined.
	pub fn add_dirent(
		&mut self,
		fs: &Ext2Fs,
		entry_inode: u32,
		name: &[u8],
		file_type: FileType,
	) -> EResult<()> {
		debug_assert_eq!(self.get_type(), FileType::Directory);
		// If the name is too long, error
		if unlikely(name.len() > NAME_MAX) {
			return Err(errno!(ENAMETOOLONG));
		}
		let mut rec_len = (dirent::NAME_OFF + name.len()).next_multiple_of(dirent::ALIGN) as u16;
		// If the entry is too large, error
		let blk_size = fs.sp.get_block_size();
		if unlikely(rec_len as u32 > blk_size) {
			return Err(errno!(ENAMETOOLONG));
		}
		if let Some((blk, off, len)) = self.find_suitable_slot(fs, rec_len)? {
			// Safe since the inode is locked
			let buf = unsafe { blk.slice_mut() };
			// Create entry
			let inner_off = (off % buf.len() as u64) as usize;
			// If not enough space is left on the block to fit another entry, use the remaining
			// space
			if inner_off + rec_len as usize + dirent::NAME_OFF >= buf.len() {
				rec_len = (buf.len() - inner_off) as u16;
			}
			Dirent::write_new(
				&mut buf[inner_off..],
				&fs.sp,
				entry_inode as _,
				rec_len,
				Some(file_type),
				name,
			)?;
			// Create free entries to cover remaining free space
			fill_free_entries(
				&mut buf[(inner_off + rec_len as usize)..(inner_off + len)],
				&fs.sp,
			)?;
			blk.mark_dirty();
		} else {
			// No suitable free entry: Fill a new block
			let blocks = self.get_blocks(&fs.sp);
			let blk_off = self.alloc_content_blk(blocks, fs)?;
			let blk = read_block(fs, blk_off as _)?;
			// Safe since the inode is locked
			let buf = unsafe { blk.slice_mut() };
			buf.fill(0);
			// Create used entry
			Dirent::write_new(buf, &fs.sp, entry_inode, rec_len, Some(file_type), name)?;
			// Create free entries to cover remaining free space
			fill_free_entries(&mut buf[rec_len as usize..], &fs.sp)?;
			self.set_size(&fs.sp, (blocks as u64 + 1) * blk_size as u64, false);
			blk.mark_dirty();
		}
		Ok(())
	}

	/// Changes the inode associated with a directory entry.
	///
	/// Arguments:
	/// - `off` is the offset of the entry to update
	/// - `inode` is the new inode to assign
	///
	/// If the entry does not exist, the function does nothing.
	///
	/// If using the value `0` for `inode`, the entry is freed. If this was the last entry in its
	/// block, the block is also freed.
	pub fn set_dirent_inode(&mut self, off: u64, inode: INode, fs: &Ext2Fs) -> EResult<()> {
		debug_assert_eq!(self.get_type(), FileType::Directory);
		let blk_size = fs.sp.get_block_size();
		let file_blk_off = off / blk_size as u64;
		let inner_off = (off % blk_size as u64) as usize;
		// Read entry's block
		let Some(disk_blk_off) = self.translate_blk_off(file_blk_off as _, fs)? else {
			return Ok(());
		};
		let blk = read_block(fs, disk_blk_off.get() as _)?;
		// Read and free entry
		let slice = unsafe { blk.slice_mut() };
		let ent = Dirent::from_slice(&mut slice[inner_off..], &fs.sp)?;
		ent.inode = inode as _;
		blk.mark_dirty();
		// If the block is now empty, free it
		if inode == 0 && is_block_empty(slice, &fs.sp)? {
			// If this is the last block, update the file's size
			if file_blk_off as u32 + 1 >= self.get_blocks(&fs.sp) {
				self.set_size(&fs.sp, file_blk_off * blk_size as u64, false);
			}
			self.free_content_blk(file_blk_off as _, fs)?;
		}
		Ok(())
	}

	/// Returns the device major and minor numbers associated with the device.
	///
	/// If the file is not a device file, the function returns `(0, 0)`.
	pub fn get_device(&self) -> (u8, u8) {
		match self.get_type() {
			FileType::BlockDevice | FileType::CharDevice => {
				let dev = self.i_block[0];
				(((dev >> 8) & 0xff) as u8, (dev & 0xff) as u8)
			}
			_ => (0, 0),
		}
	}

	/// Sets the device `major` and `minor`.
	///
	/// If the file is not a device file, the function does nothing.
	pub fn set_device(&mut self, major: u8, minor: u8) {
		if matches!(
			self.get_type(),
			FileType::BlockDevice | FileType::CharDevice
		) {
			self.i_block[0] = ((major as u32) << 8) | (minor as u32);
		}
	}
}
