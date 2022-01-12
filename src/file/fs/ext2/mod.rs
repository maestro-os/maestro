//! The ext2 filesystem is a classical filesystem used in Unix systems.
//! It is nowdays obsolete and has been replaced by ext3 and ext4.
//!
//! The filesystem divides the storage device into several substructures:
//! - Block Group: stored in the Block Group Descriptor Table (BGDT)
//! - Block: stored inside of block groups
//! - INode: represents a file in the filesystem
//! - Directory entry: an entry stored into the inode's content
//!
//! The access to an INode's data is divided into several parts, each overflowing on the next when
//! full:
//! - Direct Block Pointers: each inode has 12 of them
//! - Singly Indirect Block Pointer: a pointer to a block dedicated to storing a list of more
//! blocks to store the inode's data. The number of blocks it can store depends on the size of a
//! block.
//! - Doubly Indirect Block Pointer: a pointer to a block storing pointers to Singly Indirect Block
//! Pointers, each storing pointers to more blocks.
//! - Triply Indirect Block Pointer: a pointer to a block storing pointers to Doubly Indirect Block
//! Pointers, each storing pointers to Singly Indirect Block Pointers, each storing pointers to
//! more blocks.
//!
//! Since the size of a block pointer is 4 bytes, the maximum size of a file is:
//! `(12 * n) + ((n/4) * n) + ((n/4)^^2 * n) + ((n/4)^^3 * n)`
//! Where `n` is the size of a block.

mod block_group_descriptor;
mod directory_entry;
mod inode;

use block_group_descriptor::BlockGroupDescriptor;
use core::cmp::max;
use core::cmp::min;
use core::mem::MaybeUninit;
use core::mem::size_of;
use core::mem::size_of_val;
use core::slice;
use crate::errno::Errno;
use crate::errno;
use crate::file::File;
use crate::file::FileContent;
use crate::file::FileLocation;
use crate::file::FileType;
use crate::file::INode;
use crate::file::fs::Filesystem;
use crate::file::fs::FilesystemType;
use crate::file::path::Path;
use crate::memory::malloc;
use crate::time;
use crate::util::FailableClone;
use crate::util::IO;
use crate::util::boxed::Box;
use crate::util::container::string::String;
use crate::util::container::vec::Vec;
use crate::util::math;
use inode::Ext2INode;

// TODO Take into account user's UID/GID when allocating block/inode to handle reserved
// blocks/inodes
// TODO Document when a function writes on the storage device

/// The offset of the superblock from the beginning of the device.
const SUPERBLOCK_OFFSET: u64 = 1024;
/// The filesystem's signature.
const EXT2_SIGNATURE: u16 = 0xef53;

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

/// Optional feature: Preallocation of a specified number of blocks for each new directories.
const OPTIONAL_FEATURE_DIRECTORY_PREALLOCATION: u32 = 0x1;
/// Optional feature: AFS server
const OPTIONAL_FEATURE_AFS: u32 = 0x2;
/// Optional feature: Journal
const OPTIONAL_FEATURE_JOURNAL: u32 = 0x4;
/// Optional feature: Inodes have extended attributes
const OPTIONAL_FEATURE_INODE_EXTENDED: u32 = 0x8;
/// Optional feature: Filesystem can resize itself for larger partitions
const OPTIONAL_FEATURE_RESIZE: u32 = 0x10;
/// Optional feature: Directories use hash index
const OPTIONAL_FEATURE_HASH_INDEX: u32 = 0x20;

/// Required feature: Compression
const REQUIRED_FEATURE_COMPRESSION: u32 = 0x1;
/// Required feature: Directory entries have a type field
const REQUIRED_FEATURE_DIRECTORY_TYPE: u32 = 0x2;
/// Required feature: Filesystem needs to replay its journal
const REQUIRED_FEATURE_JOURNAL_REPLAY: u32 = 0x4;
/// Required feature: Filesystem uses a journal device
const REQUIRED_FEATURE_JOURNAL_DEVIXE: u32 = 0x8;

/// Write-required feature: Sparse superblocks and group descriptor tables
const WRITE_REQUIRED_SPARSE_SUPERBLOCKS: u32 = 0x1;
/// Write-required feature: Filesystem uses a 64-bit file size
const WRITE_REQUIRED_64_BITS: u32 = 0x2;
/// Directory contents are stored in the form of a Binary Tree.
const WRITE_REQUIRED_DIRECTORY_BINARY_TREE: u32 = 0x4;

/// Reads an object of the given type on the given device.
/// `offset` is the offset in bytes on the device.
/// `io` is the I/O interface of the device.
/// The function is marked unsafe because if the read object is invalid, the behaviour is
/// undefined.
unsafe fn read<T>(offset: u64, io: &mut dyn IO) -> Result<T, Errno> {
	let size = size_of::<T>();
	let mut obj = MaybeUninit::<T>::uninit();

	let ptr = obj.as_mut_ptr() as *mut u8;
	let buffer = slice::from_raw_parts_mut(ptr, size);
	io.read(offset, buffer)?;

	Ok(obj.assume_init())
}

/// Writes an object of the given type on the given device.
/// `obj` is the object to write.
/// `offset` is the offset in bytes on the device.
/// `io` is the I/O interface of the device.
fn write<T>(obj: &T, offset: u64, io: &mut dyn IO) -> Result<(), Errno> {
	let size = size_of_val(obj);
	let ptr = obj as *const T as *const u8;
	let buffer = unsafe {
		slice::from_raw_parts(ptr, size)
	};
	io.write(offset, buffer)?;

	Ok(())
}

/// Reads the `i`th block on the given device and writes the data onto the given buffer.
/// `i` is the offset of the block on the device.
/// `superblock` is the filesystem's superblock.
/// `io` is the I/O interface of the device.
/// `buff` is the buffer to write the data on.
/// If the block is outside of the storage's bounds, the function returns a error.
fn read_block(i: u64, superblock: &Superblock, io: &mut dyn IO, buff: &mut [u8])
	-> Result<(), Errno> {
	let blk_size = superblock.get_block_size() as u64;
	io.read(i * blk_size, buff)?;

	Ok(())
}

/// Writes the `i`th block on the given device, reading the data onto the given buffer.
/// `i` is the offset of the block on the device.
/// `superblock` is the filesystem's superblock.
/// `io` is the I/O interface of the device.
/// `buff` is the buffer to read from.
/// If the block is outside of the storage's bounds, the function returns a error.
fn write_block(i: u64, superblock: &Superblock, io: &mut dyn IO, buff: &[u8])
	-> Result<(), Errno> {
	let blk_size = superblock.get_block_size() as u64;
	io.write(i * blk_size, buff)?;

	Ok(())
}

/// The ext2 superblock structure.
#[repr(C, packed)]
pub struct Superblock {
	/// Total number of inodes in the filesystem.
	total_inodes: u32,
	/// Total number of blocks in the filesystem.
	total_blocks: u32,
	/// Number of blocks reserved for the superuser.
	superuser_blocks: u32,
	/// Total number of unallocated blocks.
	total_unallocated_blocks: u32,
	/// Total number of unallocated inodes.
	total_unallocated_inodes: u32,
	/// Block number of the block containing the superblock.
	superblock_block_number: u32,
	/// log2(block_size) - 10
	block_size_log: u32,
	/// log2(fragment_size) - 10
	fragment_size_log: u32,
	/// The number of blocks per block group.
	blocks_per_group: u32,
	/// The number of fragments per block group.
	fragments_per_group: u32,
	/// The number of inodes per block group.
	inodes_per_group: u32,
	/// The timestamp of the last mount operation.
	last_mount_timestamp: u32,
	/// The timestamp of the last write operation.
	last_write_timestamp: u32,
	/// The number of mounts since the last consistency check.
	mount_count_since_fsck: u16,
	/// The number of mounts allowed before a consistency check must be done.
	mount_count_before_fsck: u16,
	/// The ext2 signature.
	signature: u16,
	/// The filesystem's state.
	fs_state: u16,
	/// The action to perform when an error is detected.
	error_action: u16,
	/// The minor version.
	minor_version: u16,
	/// The timestamp of the last consistency check.
	last_fsck_timestamp: u32,
	/// The interval between mandatory consistency checks.
	fsck_interval: u32,
	/// The id os the operating system from which the filesystem was created.
	os_id: u32,
	/// The major version.
	major_version: u32,
	/// The UID of the user that can use reserved blocks.
	uid_reserved: u16,
	/// The GID of the group that can use reserved blocks.
	gid_reserved: u16,

	// Extended superblock fields

	/// The first non reserved inode
	first_non_reserved_inode: u32,
	/// The size of the inode structure in bytes.
	inode_size: u16,
	/// The block group containing the superblock.
	superblock_group: u16,
	/// Optional features for the implementation to support.
	optional_features: u32,
	/// Required features for the implementation to support.
	required_features: u32,
	/// Required features for the implementation to support for writing.
	write_required_features: u32,
	/// The filesystem id.
	filesystem_id: [u8; 16],
	/// The volume name.
	volume_name: [u8; 16],
	/// The path the volume was last mounted to.
	last_mount_path: [u8; 64],
	/// Used compression algorithms.
	compression_algorithms: u32,
	/// The number of blocks to preallocate for files.
	files_preallocate_count: u8,
	/// The number of blocks to preallocate for directories.
	direactories_preallocate_count: u8,
	/// Unused.
	_unused: u16,
	/// The journal ID.
	journal_id: [u8; 16],
	/// The journal inode.
	journal_inode: u32,
	/// The journal device.
	journal_device: u32,
	/// The head of orphan inodes list.
	orphan_inode_head: u32,

	/// Structure padding.
	_padding: [u8; 788],
}

impl Superblock {
	/// Creates a new instance by reading from the given device.
	pub fn read(io: &mut dyn IO) -> Result<Self, Errno> {
		unsafe {
			read::<Self>(SUPERBLOCK_OFFSET, io)
		}
	}

	/// Tells whether the superblock is valid.
	pub fn is_valid(&self) -> bool {
		self.signature == EXT2_SIGNATURE
	}

	/// Returns the size of a block.
	pub fn get_block_size(&self) -> u32 {
		math::pow2(self.block_size_log + 10) as _
	}

	/// Returns the block offset of the Block Group Descriptor Table.
	pub fn get_bgdt_offset(&self) -> u64 {
		(SUPERBLOCK_OFFSET / self.get_block_size() as u64) + 1
	}

	/// Returns the number of block groups.
	fn get_block_groups_count(&self) -> u32 {
		self.total_blocks / self.blocks_per_group
	}

	/// Returns the size of a fragment.
	pub fn get_fragment_size(&self) -> usize {
		math::pow2(self.fragment_size_log + 10) as _
	}

	/// Returns the size of an inode.
	pub fn get_inode_size(&self) -> usize {
		if self.major_version >= 1 {
			self.inode_size as _
		} else {
			128
		}
	}

	/// Returns the first inode that isn't reserved.
	pub fn get_first_available_inode(&self) -> u32 {
		if self.major_version >= 1 {
			max(self.first_non_reserved_inode, inode::ROOT_DIRECTORY_INODE + 1)
		} else {
			10
		}
	}

	/// Searches in the given bitmap block `bitmap` for the first element that is not set.
	/// The function returns the index to the element. If every elements are set, the function
	/// returns None.
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
	/// `io` is the I/O interface.
	/// `start` is the starting block.
	/// `size` is the number of entries.
	fn search_bitmap(&self, io: &mut dyn IO, start: u32, size: u32) -> Result<Option<u32>, Errno> {
		let blk_size = self.get_block_size();
		let mut buff = malloc::Alloc::<u8>::new_default(blk_size as _)?;
		let mut i = 0;

		while (i * (blk_size * 8) as u32) < size {
			let bitmap_blk_index = start + i;
			read_block(bitmap_blk_index as _, self, io, buff.get_slice_mut())?;

			if let Some(j) = Self::search_bitmap_blk(buff.get_slice()) {
				return Ok(Some(i * (blk_size * 8) as u32 + j));
			}

			i += 1;
		}

		Ok(None)
	}

	/// Changes the state of the given entry in the the given bitmap.
	/// `io` is the I/O interface.
	/// `start` is the starting block.
	/// `i` is the index of the entry to modify.
	/// `val` is the value to set the entry to.
	fn set_bitmap(&self, io: &mut dyn IO, start: u32, i: u32, val: bool) -> Result<(), Errno> {
		let blk_size = self.get_block_size();
		let mut buff = malloc::Alloc::<u8>::new_default(blk_size as _)?;

		let bitmap_blk_index = start + (i / (blk_size * 8) as u32);
		read_block(bitmap_blk_index as _, self, io, buff.get_slice_mut())?;

		let bitmap_byte_index = i / 8;
		let bitmap_bit_index = i % 8;
		if val {
			buff[bitmap_byte_index as usize] |= 1 << bitmap_bit_index;
		} else {
			buff[bitmap_byte_index as usize] &= !(1 << bitmap_bit_index);
		}

		write_block(bitmap_blk_index as _, self, io, buff.get_slice())
	}

	/// Returns the id of a free inode in the filesystem.
	/// `io` is the I/O interface.
	pub fn get_free_inode(&self, io: &mut dyn IO) -> Result<u32, Errno> {
		for i in 0..self.get_block_groups_count() {
			let bgd = BlockGroupDescriptor::read(i as _, self, io)?;
			if bgd.unallocated_inodes_number > 0 {
				if let Some(j) = self.search_bitmap(io, bgd.inode_usage_bitmap_addr,
					self.inodes_per_group)? {
					return Ok(i * self.inodes_per_group + j + 1);
				}
			}
		}

		Err(errno::ENOSPC)
	}

	/// Marks the inode `inode` used on the filesystem.
	/// `io` is the I/O interface.
	/// `inode` is the inode number.
	/// `directory` tells whether the inode is allocated for a directory.
	/// If the inode is already marked as used, the behaviour is undefined.
	pub fn mark_inode_used(&self, io: &mut dyn IO, inode: u32, directory: bool)
		-> Result<(), Errno> {
		debug_assert!(inode >= 1);

		let group = (inode - 1) / self.inodes_per_group;
		let mut bgd = BlockGroupDescriptor::read(group, self, io)?;
		bgd.unallocated_inodes_number -= 1;
		if directory {
			bgd.directories_number += 1;
		}

		let bitfield_index = (inode - 1) % self.inodes_per_group;
		self.set_bitmap(io, bgd.inode_usage_bitmap_addr, bitfield_index, true)?;

		bgd.write(group, self, io)
	}

	/// Marks the inode `inode` available on the filesystem.
	/// `io` is the I/O interface.
	/// `inode` is the inode number.
	/// `directory` tells whether the inode is allocated for a directory.
	/// If the inode is already marked as free, the behaviour is undefined.
	pub fn free_inode(&self, io: &mut dyn IO, inode: u32, directory: bool) -> Result<(), Errno> {
		debug_assert!(inode >= 1);

		let group = (inode - 1) / self.inodes_per_group;
		let mut bgd = BlockGroupDescriptor::read(group, self, io)?;
		bgd.unallocated_inodes_number += 1;
		if directory {
			bgd.directories_number -= 1;
		}

		let bitfield_index = (inode - 1) % self.inodes_per_group;
		self.set_bitmap(io, bgd.inode_usage_bitmap_addr, bitfield_index, false)?;

		bgd.write(group, self, io)
	}

	/// Returns the id of a free block in the filesystem.
	/// `io` is the I/O interface.
	pub fn get_free_block(&self, io: &mut dyn IO) -> Result<u32, Errno> {
		for i in 0..self.get_block_groups_count() {
			let bgd = BlockGroupDescriptor::read(i as _, self, io)?;
			if bgd.unallocated_blocks_number > 0 {
				if let Some(j) = self.search_bitmap(io, bgd.block_usage_bitmap_addr,
					self.blocks_per_group)? {
					return Ok(i * self.blocks_per_group + j);
				}
			}
		}

		Err(errno::ENOSPC)
	}

	/// Marks the block `blk` used on the filesystem.
	/// `io` is the I/O interface.
	/// `blk` is the block number.
	pub fn mark_block_used(&self, io: &mut dyn IO, blk: u32) -> Result<(), Errno> {
		let group = blk / self.blocks_per_group;
		let mut bgd = BlockGroupDescriptor::read(group, self, io)?;
		bgd.unallocated_blocks_number -= 1;

		let bitfield_index = blk % self.blocks_per_group;
		self.set_bitmap(io, bgd.block_usage_bitmap_addr, bitfield_index, true)?;

		bgd.write(group, self, io)
	}

	/// Marks the block `blk` available on the filesystem.
	/// `io` is the I/O interface.
	/// `blk` is the block number.
	pub fn free_block(&self, io: &mut dyn IO, blk: u32) -> Result<(), Errno> {
		let group = blk / self.blocks_per_group;
		let mut bgd = BlockGroupDescriptor::read(group, self, io)?;
		bgd.unallocated_blocks_number += 1;

		let bitfield_index = blk % self.blocks_per_group;
		self.set_bitmap(io, bgd.block_usage_bitmap_addr, bitfield_index, false)?;

		bgd.write(group, self, io)
	}

	/// Writes the superblock on the device.
	pub fn write(&self, io: &mut dyn IO) -> Result<(), Errno> {
		write::<Self>(self, SUPERBLOCK_OFFSET, io)
	}
}

/// Structure representing a instance of the ext2 filesystem.
struct Ext2Fs {
	/// The path at which the filesystem is mounted.
	mountpath: Path,

	/// The filesystem's superblock.
	superblock: Superblock,

	/// Tells whether the filesystem is mounted in read-only.
	readonly: bool,
}

impl Ext2Fs {
	/// Creates a new instance.
	/// If the filesystem cannot be mounted, the function returns an Err.
	/// `mountpath` is the path on which the filesystem is mounted.
	/// `readonly` tells whether the filesystem is mounted in read-only.
	fn new(mut superblock: Superblock, io: &mut dyn IO, mountpath: Path, readonly: bool)
		-> Result<Self, Errno> {
		debug_assert!(superblock.is_valid());

		// Checking the filesystem doesn't require features that are not implemented by the driver
		if superblock.major_version >= 1 {
			// TODO Implement journal
			let unsupported_required_features = REQUIRED_FEATURE_COMPRESSION
				| REQUIRED_FEATURE_JOURNAL_REPLAY
				| REQUIRED_FEATURE_JOURNAL_DEVIXE;

			if superblock.required_features & unsupported_required_features != 0 {
				// TODO Log?
				return Err(errno::EINVAL);
			}

			// TODO Implement
			let unsupported_write_features = WRITE_REQUIRED_DIRECTORY_BINARY_TREE;

			if !readonly && superblock.write_required_features & unsupported_write_features != 0 {
				// TODO Log?
				return Err(errno::EROFS);
			}
		}

		let timestamp = time::get();
		if superblock.mount_count_since_fsck >= superblock.mount_count_before_fsck {
			return Err(errno::EINVAL);
		}
		// TODO
		/*if timestamp >= superblock.last_fsck_timestamp + superblock.fsck_interval {
			return Err(errno::EINVAL);
		}*/

		superblock.mount_count_since_fsck += 1;

		// Setting the last mount path
		{
			let mountpath_str = mountpath.as_string()?;
			let mountpath_bytes = mountpath_str.as_bytes();

			let mut i = 0;
			while i < min(mountpath_bytes.len(), superblock.last_mount_path.len()) {
				superblock.last_mount_path[i] = mountpath_bytes[i];
				i += 1;
			}
			while i < superblock.last_mount_path.len() {
				superblock.last_mount_path[i] = 0;
				i += 1;
			}
		}

		// Setting the last mount timestamp
		if let Some(timestamp) = timestamp {
			superblock.last_mount_timestamp = timestamp;
		}

		superblock.write(io)?;

		Ok(Self {
			mountpath,

			superblock,

			readonly,
		})
	}
}

// TODO Update the write timestamp when the fs is written (take mount flags into account)
impl Filesystem for Ext2Fs {
	fn get_name(&self) -> &[u8] {
		b"ext2"
	}

	fn is_readonly(&self) -> bool {
		self.readonly
	}

	fn must_cache(&self) -> bool {
		true
	}

	fn get_inode(&mut self, io: &mut dyn IO, path: Path) -> Result<INode, Errno> {
		debug_assert!(path.is_absolute());

		let mut inode_index = inode::ROOT_DIRECTORY_INODE;
		for i in 0..path.get_elements_count() {
			let inode = Ext2INode::read(inode_index, &self.superblock, io)?;
			if inode.get_type() != FileType::Directory {
				return Err(errno::ENOTDIR);
			}

			let name = &path[i];
			if let Some(entry) = inode.get_directory_entry(name.as_bytes(), &self.superblock,
				io)? {
				inode_index = entry.get_inode();
			} else {
				return Err(errno::ENOENT);
			}
		}

		Ok(inode_index)
	}

	fn load_file(&mut self, io: &mut dyn IO, inode: INode, name: String) -> Result<File, Errno> {
		let inode_ = Ext2INode::read(inode, &self.superblock, io)?;
		let file_type = inode_.get_type();

		let file_content = match file_type {
			FileType::Regular => FileContent::Regular,
			FileType::Directory => {
				let mut subfiles = Vec::new();
				let mut err = None;

				inode_.foreach_directory_entry(| _, entry | {
					match String::from(entry.get_name(&self.superblock)) {
						Ok(s) => {
							if let Err(e) = subfiles.push(s) {
								err = Some(e);
								false
							} else {
								true
							}
						},
						Err(e) => {
							err = Some(e);
							false
						},
					}
				}, &self.superblock, io)?;

				if let Some(e) = err {
					return Err(e);
				}

				FileContent::Directory(subfiles)
			},
			FileType::Link => {
				// TODO Read symlink path
				todo!();
			},
			FileType::Fifo => {
				// TODO
				todo!();
			},
			FileType::Socket => {
				// TODO
				todo!();
			},
			FileType::BlockDevice => {
				let (major, minor) = inode_.get_device();

				FileContent::BlockDevice {
					major: major as _,
					minor: minor as _,
				}
			},
			FileType::CharDevice => {
				let (major, minor) = inode_.get_device();

				FileContent::CharDevice {
					major: major as _,
					minor: minor as _,
				}
			},
		};

		let mut file = File::new(name, file_content, inode_.uid, inode_.gid,
			inode_.get_permissions())?;
		file.set_location(Some(FileLocation::new(self.mountpath.failable_clone()?, inode)));
		file.set_ctime(inode_.ctime);
		file.set_mtime(inode_.mtime);
		file.set_atime(inode_.atime);
		file.set_size(inode_.get_size(&self.superblock));

		Ok(file)
	}

	// TODO Check if the file exists. If it does, return EEXIST
	fn add_file(&mut self, io: &mut dyn IO, parent_inode: INode, mut file: File)
		-> Result<File, Errno> {
		if self.readonly {
			return Err(errno::EROFS);
		}

		let mut parent = Ext2INode::read(parent_inode, &self.superblock, io)?;

		// Checking the parent file is a directory
		if parent.get_type() != FileType::Directory {
			return Err(errno::ENOTDIR);
		}

		let mut inode = Ext2INode {
			mode: Ext2INode::get_file_mode(&file),
			uid: file.get_uid(),
			size_low: 0,
			ctime: file.get_ctime(),
			mtime: file.get_mtime(),
			atime: file.get_atime(),
			dtime: 0,
			gid: file.get_gid(),
			hard_links_count: 1,
			used_sectors: 0,
			flags: 0,
			os_specific_0: 0,
			direct_block_ptrs: [0; inode::DIRECT_BLOCKS_COUNT as usize],
			singly_indirect_block_ptr: 0,
			doubly_indirect_block_ptr: 0,
			triply_indirect_block_ptr: 0,
			generation: 0,
			extended_attributes_block: 0,
			size_high: 0,
			fragment_addr: 0,
			os_specific_1: [0; 12],
		};
		// TODO When adding a directory, add '.' and '..' in it?
		match file.get_file_content() {
			FileContent::Link(_target) => {
				// TODO Write symlink target
				todo!();
			},

			FileContent::BlockDevice { major, minor }
				| FileContent::CharDevice { major, minor } => {
				if *major > (u8::MAX as u32) || *minor > (u8::MAX as u32) {
					return Err(errno::ENODEV);
				}

				inode.set_device(*major as u8, *minor as u8);
			},

			_ => {},
		}

		let inode_index = self.superblock.get_free_inode(io)?;
		inode.write(inode_index, &self.superblock, io)?;
		let dir = file.get_file_type() == FileType::Directory;
		self.superblock.mark_inode_used(io, inode_index, dir)?;

		parent.add_dirent(&self.superblock, io, inode_index, file.get_name(),
			file.get_file_type())?;
		parent.write(parent_inode, &self.superblock, io)?;

		file.set_location(Some(FileLocation::new(self.mountpath.failable_clone()?, inode_index)));
		Ok(file)
	}

	fn remove_file(&mut self, io: &mut dyn IO, parent_inode: INode, _name: &String)
		-> Result<(), Errno> {
		if self.readonly {
			return Err(errno::EROFS);
		}

		debug_assert!(parent_inode >= 1);

		let parent = Ext2INode::read(parent_inode, &self.superblock, io)?;
		debug_assert_eq!(parent.get_type(), FileType::Directory);

		// TODO
		todo!();
	}

	fn read_node(&mut self, io: &mut dyn IO, inode: INode, off: u64, buf: &mut [u8])
		-> Result<usize, Errno> {
		debug_assert!(inode >= 1);

		let inode_ = Ext2INode::read(inode, &self.superblock, io)?;
		inode_.read_content(off, buf, &self.superblock, io)
	}

	fn write_node(&mut self, io: &mut dyn IO, inode: INode, off: u64, buf: &[u8])
		-> Result<(), Errno> {
		if self.readonly {
			return Err(errno::EROFS);
		}

		debug_assert!(inode >= 1);

		let mut inode_ = Ext2INode::read(inode, &self.superblock, io)?;
		inode_.write_content(off, buf, &self.superblock, io)?;
		inode_.write(inode, &self.superblock, io)
	}
}

/// Structure representing the ext2 filesystem type.
pub struct Ext2FsType {}

impl FilesystemType for Ext2FsType {
	fn get_name(&self) -> &[u8] {
		b"ext2"
	}

	fn detect(&self, io: &mut dyn IO) -> Result<bool, Errno> {
		Ok(Superblock::read(io)?.is_valid())
	}

	fn create_filesystem(&self, io: &mut dyn IO) -> Result<Box<dyn Filesystem>, Errno> {
		let timestamp = time::get().unwrap_or(0);

		let blocks_count = (io.get_size() / DEFAULT_BLOCK_SIZE) as u32;
		let groups_count = blocks_count / DEFAULT_BLOCKS_PER_GROUP;

		let inodes_count = groups_count * DEFAULT_INODES_PER_GROUP;

		let block_usage_bitmap_size = math::ceil_division(DEFAULT_BLOCKS_PER_GROUP,
			(DEFAULT_BLOCK_SIZE * 8) as _);
		let inode_usage_bitmap_size = math::ceil_division(DEFAULT_INODES_PER_GROUP,
			(DEFAULT_BLOCK_SIZE * 8) as _);
		let inodes_table_size = math::ceil_division(DEFAULT_INODES_PER_GROUP
			* DEFAULT_INODE_SIZE as u32,
			(DEFAULT_BLOCK_SIZE * 8) as _);

		let superblock = Superblock {
			total_inodes: inodes_count,
			total_blocks: blocks_count,
			superuser_blocks: 0,
			total_unallocated_blocks: blocks_count,
			total_unallocated_inodes: inodes_count,
			superblock_block_number: (SUPERBLOCK_OFFSET / DEFAULT_BLOCK_SIZE) as _,
			block_size_log: (math::log2(DEFAULT_BLOCK_SIZE as usize) - 10) as _,
			fragment_size_log: 0,
			blocks_per_group: DEFAULT_BLOCKS_PER_GROUP,
			fragments_per_group: 0,
			inodes_per_group: DEFAULT_INODES_PER_GROUP,
			last_mount_timestamp: timestamp,
			last_write_timestamp: timestamp,
			mount_count_since_fsck: 0,
			mount_count_before_fsck: DEFAULT_MOUNT_COUNT_BEFORE_FSCK,
			signature: EXT2_SIGNATURE,
			fs_state: FS_STATE_CLEAN,
			error_action: ERR_ACTION_READ_ONLY,
			minor_version: DEFAULT_MINOR,
			last_fsck_timestamp: timestamp,
			fsck_interval: DEFAULT_FSCK_INTERVAL,
			os_id: 0xdeadbeef,
			major_version: DEFAULT_MAJOR,
			uid_reserved: 0,
			gid_reserved: 0,

			first_non_reserved_inode: 11,
			inode_size: DEFAULT_INODE_SIZE,
			superblock_group: ((SUPERBLOCK_OFFSET / DEFAULT_BLOCK_SIZE) as u32
				/ DEFAULT_BLOCKS_PER_GROUP) as _,
			optional_features: 0,
			required_features: 0,
			write_required_features: WRITE_REQUIRED_64_BITS,
			filesystem_id: [0; 16], // TODO
			volume_name: [0; 16], // TODO
			last_mount_path: [0; 64], // TODO
			compression_algorithms: 0,
			files_preallocate_count: 0,
			direactories_preallocate_count: 0,
			_unused: 0,
			journal_id: [0; 16], // TODO
			journal_inode: 0, // TODO
			journal_device: 0, // TODO
			orphan_inode_head: 0, // TODO

			_padding: [0; 788],
		};
		superblock.write(io)?;

		let blk_size = superblock.get_block_size() as u32;
		let bgdt_offset = superblock.get_bgdt_offset();
		let bgdt_size = math::ceil_division(groups_count
			* size_of::<BlockGroupDescriptor>() as u32, blk_size);
		let bgdt_end = bgdt_offset + bgdt_size as u64;

		for i in 0..groups_count {
			let metadata_off = max(i * DEFAULT_BLOCKS_PER_GROUP, bgdt_end as u32);
			let metadata_size = block_usage_bitmap_size + inode_usage_bitmap_size
				+ inodes_table_size;
			debug_assert!(bgdt_end + metadata_size as u64 <= DEFAULT_BLOCKS_PER_GROUP as u64);

			let block_usage_bitmap_addr = metadata_off;
			let inode_usage_bitmap_addr = metadata_off + block_usage_bitmap_size;
			let inode_table_start_addr = metadata_off + block_usage_bitmap_size
				+ inode_usage_bitmap_size;

			let bgd = BlockGroupDescriptor {
				block_usage_bitmap_addr,
				inode_usage_bitmap_addr,
				inode_table_start_addr,
				unallocated_blocks_number: DEFAULT_BLOCKS_PER_GROUP as _,
				unallocated_inodes_number: DEFAULT_INODES_PER_GROUP as _,
				directories_number: 0,

				_padding: [0; 14],
			};
			bgd.write(i, &superblock, io)?;
		}

		superblock.mark_block_used(io, 0)?;

		let superblock_blk_offset = SUPERBLOCK_OFFSET as u32 / blk_size;
		superblock.mark_block_used(io, superblock_blk_offset)?;

		let bgdt_size = size_of::<BlockGroupDescriptor>() as u32 * groups_count;
		let bgdt_blk_count = math::ceil_division(bgdt_size, blk_size);
		for j in 0..bgdt_blk_count {
			let blk = bgdt_offset + j as u64;
			superblock.mark_block_used(io, blk as _)?;
		}

		for i in 0..groups_count {
			let bgd = BlockGroupDescriptor::read(i, &superblock, io)?;

			for j in 0..block_usage_bitmap_size {
				let blk = bgd.block_usage_bitmap_addr + j;
				superblock.mark_block_used(io, blk)?;
			}

			for j in 0..inode_usage_bitmap_size {
				let inode = bgd.inode_usage_bitmap_addr + j;
				superblock.mark_block_used(io, inode)?;
			}

			for j in 0..inodes_table_size {
				let blk = bgd.inode_table_start_addr + j;
				superblock.mark_block_used(io, blk)?;
			}
		}

		for i in 1..superblock.get_first_available_inode() {
			let is_dir = i == inode::ROOT_DIRECTORY_INODE;
			superblock.mark_inode_used(io, i, is_dir)?;
		}

		let root_dir = Ext2INode {
			mode: inode::INODE_TYPE_DIRECTORY | inode::ROOT_DIRECTORY_DEFAULT_MODE,
			uid: 0,
			size_low: 0,
			ctime: timestamp,
			mtime: timestamp,
			atime: timestamp,
			dtime: 0,
			gid: 0,
			hard_links_count: 1,
			used_sectors: 0,
			flags: 0,
			os_specific_0: 0,
			direct_block_ptrs: [0; inode::DIRECT_BLOCKS_COUNT as usize],
			singly_indirect_block_ptr: 0,
			doubly_indirect_block_ptr: 0,
			triply_indirect_block_ptr: 0,
			generation: 0,
			extended_attributes_block: 0,
			size_high: 0,
			fragment_addr: 0,
			os_specific_1: [0; 12],
		};
		root_dir.write(inode::ROOT_DIRECTORY_INODE, &superblock, io)?;

		let fs = Ext2Fs::new(superblock, io, Path::root(), true)?;
		Ok(Box::new(fs)?)
	}

	fn load_filesystem(&self, io: &mut dyn IO, mountpath: Path, readonly: bool)
		-> Result<Box<dyn Filesystem>, Errno> {
		let superblock = Superblock::read(io)?;
		let fs = Ext2Fs::new(superblock, io, mountpath, readonly)?;

		Ok(Box::new(fs)? as _)
	}
}
