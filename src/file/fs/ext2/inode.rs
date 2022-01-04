//! An inode represents a file in the filesystem.

use core::cmp::max;
use core::cmp::min;
use core::mem::size_of;
use core::ptr::copy_nonoverlapping;
use core::slice;
use crate::errno::Errno;
use crate::errno;
use crate::file::File;
use crate::file::FileType;
use crate::file;
use crate::memory::malloc;
use crate::util::IO;
use crate::util::boxed::Box;
use crate::util::container::string::String;
use crate::util::math;
use super::Superblock;
use super::block_group_descriptor::BlockGroupDescriptor;
use super::directory_entry::DirectoryEntry;
use super::read;
use super::read_block;
use super::write;
use super::write_block;

/// The maximum number of direct blocks for each inodes.
pub const DIRECT_BLOCKS_COUNT: u8 = 12;

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

/// User: Read, Write and Execute.
const INODE_PERMISSION_IRWXU: u16 = 0o0700;
/// User: Read.
const INODE_PERMISSION_IRUSR: u16 = 0o0400;
/// User: Write.
const INODE_PERMISSION_IWUSR: u16 = 0o0200;
/// User: Execute.
const INODE_PERMISSION_IXUSR: u16 = 0o0100;
/// Group: Read, Write and Execute.
const INODE_PERMISSION_IRWXG: u16 = 0o0070;
/// Group: Read.
const INODE_PERMISSION_IRGRP: u16 = 0o0040;
/// Group: Write.
const INODE_PERMISSION_IWGRP: u16 = 0o0020;
/// Group: Execute.
const INODE_PERMISSION_IXGRP: u16 = 0o0010;
/// Other: Read, Write and Execute.
const INODE_PERMISSION_IRWXO: u16 = 0o0007;
/// Other: Read.
const INODE_PERMISSION_IROTH: u16 = 0o0004;
/// Other: Write.
const INODE_PERMISSION_IWOTH: u16 = 0o0002;
/// Other: Execute.
const INODE_PERMISSION_IXOTH: u16 = 0o0001;
/// Setuid.
const INODE_PERMISSION_ISUID: u16 = 0o4000;
/// Setgid.
const INODE_PERMISSION_ISGID: u16 = 0o2000;
/// Sticky bit.
const INODE_PERMISSION_ISVTX: u16 = 0o1000;

/// Secure deletion
const INODE_FLAG_SECURE_DELETION: u32 = 0x00001;
/// Keep a copy of data when deleted
const INODE_FLAG_DELETE_COPY: u32 = 0x00002;
/// File compression
const INODE_FLAG_COMPRESSION: u32 = 0x00004;
/// Synchronous updates
const INODE_FLAG_SYNC: u32 = 0x00008;
/// Immutable file
const INODE_FLAG_IMMUTABLE: u32 = 0x00010;
/// Append only
const INODE_FLAG_APPEND_ONLY: u32 = 0x00020;
/// File is not included in 'dump' command
const INODE_FLAG_NODUMP: u32 = 0x00040;
/// Last accessed time should not updated
const INODE_FLAG_ATIME_NOUPDATE: u32 = 0x00080;
/// Hash indexed directory
const INODE_FLAG_HASH_INDEXED: u32 = 0x10000;
/// AFS directory
const INODE_FLAG_AFS_DIRECTORY: u32 = 0x20000;
/// Journal file data
const INODE_FLAG_JOURNAL_FILE: u32 = 0x40000;

/// The inode of the root directory.
pub const ROOT_DIRECTORY_INODE: u32 = 2;
/// The root directory's default mode.
pub const ROOT_DIRECTORY_DEFAULT_MODE: u16 = INODE_PERMISSION_IRWXU
	| INODE_PERMISSION_IRGRP | INODE_PERMISSION_IXGRP
	| INODE_PERMISSION_IROTH | INODE_PERMISSION_IXOTH;

/// An inode represents a file in the filesystem. The name of the file is not included in the inode
/// but in the directory entry associated with it since several entries can refer to the same
/// inode (hard links).
#[repr(C, packed)]
pub struct Ext2INode {
	/// Type and permissions.
	pub mode: u16,
	/// User ID.
	pub uid: u16,
	/// Lower 32 bits of size in bytes.
	pub size_low: u32,
	/// Timestamp of the last modification of the metadata.
	pub ctime: u32,
	/// Timestamp of the last modification of the content.
	pub mtime: u32,
	/// Timestamp of the last access.
	pub atime: u32,
	/// Timestamp of the deletion.
	pub dtime: u32,
	/// Group ID.
	pub gid: u16,
	/// The number of hard links to this inode.
	pub hard_links_count: u16,
	/// The number of sectors used by this inode.
	pub used_sectors: u32,
	/// INode flags.
	pub flags: u32,
	/// OS-specific value.
	pub os_specific_0: u32,
	/// Direct block pointers.
	pub direct_block_ptrs: [u32; DIRECT_BLOCKS_COUNT as usize],
	/// Simply indirect block pointer.
	pub singly_indirect_block_ptr: u32,
	/// Doubly indirect block pointer.
	pub doubly_indirect_block_ptr: u32,
	/// Triply indirect block pointer.
	pub triply_indirect_block_ptr: u32,
	/// Generation number.
	pub generation: u32,
	/// The file's ACL.
	pub extended_attributes_block: u32,
	/// Higher 32 bits of size in bytes.
	pub size_high: u32,
	/// Block address of fragment.
	pub fragment_addr: u32,
	/// OS-specific value.
	pub os_specific_1: [u8; 12],
}

impl Ext2INode {
	/// Returns the offset of the inode on the disk in bytes.
	/// `i` is the inode's index (starting at `1`).
	/// `superblock` is the filesystem's superblock.
	/// `io` is the I/O interface.
	fn get_disk_offset(i: u32, superblock: &Superblock, io: &mut dyn IO) -> Result<u64, Errno> {
		// Checking the inode is correct
		if i == 0 {
			return Err(errno::EINVAL);
		}

		let blk_size = superblock.get_block_size();
		let inode_size = superblock.get_inode_size();

		// The block group the inode is located in
		let blk_grp = (i - 1) / superblock.inodes_per_group;
		// The offset of the inode in the block group's bitfield
		let inode_grp_off = (i - 1) % superblock.inodes_per_group;
		// The offset of the inode's block
		let inode_table_blk_off = (inode_grp_off * inode_size as u32) / (blk_size as u32);
		// The offset of the inode in the block
		let inode_blk_off = ((i - 1) * inode_size as u32) % blk_size;

		let bgd = BlockGroupDescriptor::read(blk_grp, superblock, io)?;
		// The block containing the inode
		let blk = bgd.inode_table_start_addr + inode_table_blk_off;

		// The offset of the inode on the disk
		let inode_offset = (blk as u64 * blk_size as u64) + inode_blk_off as u64;
		Ok(inode_offset)
	}

	/// Returns the mode for the given file `file`.
	pub fn get_file_mode(file: &File) -> u16 {
		let t = match file.get_file_type() {
			FileType::Fifo => INODE_TYPE_FIFO,
			FileType::CharDevice => INODE_TYPE_CHAR_DEVICE,
			FileType::Directory => INODE_TYPE_DIRECTORY,
			FileType::BlockDevice => INODE_TYPE_BLOCK_DEVICE,
			FileType::Regular => INODE_TYPE_REGULAR,
			FileType::Link => INODE_TYPE_SYMLINK,
			FileType::Socket => INODE_TYPE_SOCKET,
		};

		file.get_mode() as u16 | t
	}

	/// Reads the `i`th inode from the given device. The index `i` starts at `1`.
	/// `superblock` is the filesystem's superblock.
	/// `io` is the I/O interface.
	pub fn read(i: u32, superblock: &Superblock, io: &mut dyn IO) -> Result<Self, Errno> {
		let off = Self::get_disk_offset(i, superblock, io)?;

		unsafe {
			read::<Self>(off, io)
		}
	}

	/// Returns the type of the file.
	pub fn get_type(&self) -> FileType {
		let file_type = self.mode & 0xf000;

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

	/// Returns the permissions of the file.
	#[inline]
	pub fn get_permissions(&self) -> file::Mode {
		self.mode as file::Mode & 0x0fff
	}

	/// Returns the size of the file.
	/// `superblock` is the filesystem's superblock.
	pub fn get_size(&self, superblock: &Superblock) -> u64 {
		let has_version = superblock.major_version >= 1;
		let has_feature = superblock.write_required_features & super::WRITE_REQUIRED_64_BITS != 0;

		if has_version && has_feature {
			((self.size_high as u64) << 32) | (self.size_low as u64)
		} else {
			self.size_low as u64
		}
	}

	/// Sets the file's size.
	/// `superblock` is the filesystem's superblock.
	/// `size` is the file's size.
	fn set_size(&mut self, superblock: &Superblock, size: u64) {
		let has_version = superblock.major_version >= 1;
		let has_feature = superblock.write_required_features & super::WRITE_REQUIRED_64_BITS != 0;

		if has_version && has_feature {
			self.size_high = ((size >> 32) & 0xffff) as u32;
			self.size_low = (size & 0xffff) as u32;
		} else {
			self.size_low = size as u32;
		}
	}

	/// Turns a block offset into an Option./ Namely, if the block offset is zero, the function
	/// returns None.
	fn blk_offset_to_option(blk: u32) -> Option<u32> {
		if blk != 0 {
			Some(blk)
		} else {
			None
		}
	}

	/// Resolves block indirections.
	/// `n` is the number of indirections to resolve.
	/// `begin` is the beginning block.
	/// `off` is the offset of the block relative to the specified beginning block.
	/// `superblock` is the filesystem's superblock.
	/// `io` is the I/O interface.
	/// If the block doesn't exist, the function returns None.
	fn resolve_indirections(n: u8, begin: u32, off: u32, superblock: &Superblock,
		io: &mut dyn IO) -> Result<Option<u32>, Errno> {
		let blk_size = superblock.get_block_size();
		let entries_per_blk = blk_size / size_of::<u32>() as u32;

		let mut b = begin;
		for i in (0..n).rev() {
			let inner_index = off / math::pow(entries_per_blk as u32, i as _);
			let inner_off = inner_index as u64 * size_of::<u32>() as u64;
			let byte_off = (b as u64 * blk_size as u64) + inner_off as u64;
			b = unsafe {
				read::<u32>(byte_off, io)?
			};

			if b == 0 {
				break;
			}
		}

		Ok(Self::blk_offset_to_option(b))
	}

	// TODO Check correctness
	/// Allocates a new block for the content of the file through block indirections.
	/// `n` is the number of indirections to resolve.
	/// `begin` is the beginning block.
	/// `off` is the offset of the block relative to the specified beginning block.
	/// `superblock` is the filesystem's superblock.
	/// `io` is the I/O interface.
	fn indirections_alloc(n: u8, begin: u32, off: u32, superblock: &Superblock,
		io: &mut dyn IO) -> Result<u32, Errno> {
		let blk_size = superblock.get_block_size();
		let entries_per_blk = blk_size / size_of::<u32>() as u32;

		let mut b = begin;
		for i in (0..(n + 1)).rev() {
			let inner_index = off / math::pow(entries_per_blk as u32, i as _);
			let inner_off = inner_index as u64 * size_of::<u32>() as u64;
			let byte_off = (b as u64 * blk_size as u64) + inner_off as u64;

			if b == 0 {
				let blk = superblock.get_free_block(io)?;
				superblock.mark_block_used(io, blk)?;
				write::<u32>(&blk, byte_off, io)?;
			} else {
				b = unsafe {
					read::<u32>(byte_off, io)?
				};
			}
		}

		Ok(b)
	}

	/// Returns the block id of the node's content block at the given offset `i`.
	/// `i` is the block offset in the node's content.
	/// `superblock` is the filesystem's superblock.
	/// `io` is the I/O interface.
	/// If the block doesn't exist, the function returns None.
	fn get_content_block_off(&self, i: u32, superblock: &Superblock, io: &mut dyn IO)
		-> Result<Option<u32>, Errno> {
		let blk_size = superblock.get_block_size();
		let entries_per_blk = blk_size / size_of::<u32>() as u32;

		if i < DIRECT_BLOCKS_COUNT as u32 {
			Ok(Self::blk_offset_to_option(self.direct_block_ptrs[i as usize]))
		} else if i < DIRECT_BLOCKS_COUNT as u32 + entries_per_blk {
			let target = i - DIRECT_BLOCKS_COUNT as u32;
			Self::resolve_indirections(1, self.singly_indirect_block_ptr, target, superblock, io)
		} else if i < DIRECT_BLOCKS_COUNT as u32 + (entries_per_blk * entries_per_blk) {
			let target = (i - DIRECT_BLOCKS_COUNT as u32 - entries_per_blk) as u32;
			Self::resolve_indirections(2, self.doubly_indirect_block_ptr, target, superblock, io)
		} else {
			#[allow(clippy::suspicious_operation_groupings)]
			let target = i - DIRECT_BLOCKS_COUNT as u32 - (entries_per_blk * entries_per_blk);
			Self::resolve_indirections(3, self.triply_indirect_block_ptr, target, superblock, io)
		}
	}

	/// Allocates a block for the node's content block at the given offset `i`.
	/// If the block is already allocated, the function does nothing.
	/// `i` is the block offset in the node's content.
	/// `superblock` is the filesystem's superblock.
	/// `io` is the I/O interface.
	/// On success, the function returns the allocated final block offset.
	fn alloc_content_block(&mut self, i: u32, superblock: &Superblock, io: &mut dyn IO)
		-> Result<u32, Errno> {
		let blk_size = superblock.get_block_size();
		let entries_per_blk = blk_size / size_of::<u32>() as u32;

		if i < DIRECT_BLOCKS_COUNT as u32 {
			let blk = superblock.get_free_block(io)?;
			self.direct_block_ptrs[i as usize] = blk;
			superblock.mark_block_used(io, blk)?;

			Ok(blk)
		} else if i < DIRECT_BLOCKS_COUNT as u32 + entries_per_blk {
			let target = i - DIRECT_BLOCKS_COUNT as u32;
			Self::indirections_alloc(1, self.singly_indirect_block_ptr, target, superblock, io)
		} else if i < DIRECT_BLOCKS_COUNT as u32 + (entries_per_blk * entries_per_blk) {
			let target = i - DIRECT_BLOCKS_COUNT as u32 - entries_per_blk;
			Self::indirections_alloc(2, self.doubly_indirect_block_ptr, target, superblock, io)
		} else {
			#[allow(clippy::suspicious_operation_groupings)]
			let target = i - DIRECT_BLOCKS_COUNT as u32 - (entries_per_blk * entries_per_blk);
			Self::indirections_alloc(3, self.triply_indirect_block_ptr, target, superblock, io)
		}
	}

	/// Frees a content block at block offset `i` in file.
	/// If the block isn't allocated, the function does nothing.
	/// `i` is the id of the block.
	/// `superblock` is the filesystem's superblock.
	/// `io` is the I/O interface.
	fn free_content_block(&mut self, _i: usize, _superblock: &Superblock,
		_io: &mut dyn IO) {
		// TODO
		todo!();
	}

	/// Reads the content of the inode.
	/// `off` is the offset at which the inode is read.
	/// `buff` is the buffer in which the data is to be written.
	/// `superblock` is the filesystem's superblock.
	/// `io` is the I/O interface.
	/// The function returns the number of bytes that have been read.
	pub fn read_content(&self, off: u64, buff: &mut [u8], superblock: &Superblock,
		io: &mut dyn IO) -> Result<usize, Errno> {
		let size = self.get_size(&superblock);
		if off > size {
			return Err(errno::EINVAL);
		}

		let blk_size = superblock.get_block_size();
		let mut blk_buff = malloc::Alloc::<u8>::new_default(blk_size as usize)?;

		let mut i = 0;
		let max = min(buff.len(), (size - off) as usize);
		while i < max {
			let blk_off = (off + i as u64) / blk_size as u64;
			let blk_inner_off = ((off + i as u64) % blk_size as u64) as usize;
			let blk_off = self.get_content_block_off(blk_off as _, superblock, io)?.unwrap();
			read_block(blk_off as _, superblock, io, blk_buff.get_slice_mut())?;

			let len = min(buff.len() - i, (blk_size - blk_inner_off as u32) as usize);
			unsafe { // Safe because staying in range
				copy_nonoverlapping(&blk_buff.get_slice()[blk_inner_off] as *const u8,
					&mut buff[i] as *mut u8,
					len);
			}

			i += len;
		}

		Ok(min(i, max))
	}

	/// Writes the content of the inode.
	/// `off` is the offset at which the inode is written.
	/// `buff` is the buffer in which the data is to be written.
	/// `superblock` is the filesystem's superblock.
	/// `io` is the I/O interface.
	/// The function returns the number of bytes that have been written.
	pub fn write_content(&mut self, off: u64, buff: &[u8], superblock: &Superblock,
		io: &mut dyn IO) -> Result<(), Errno> {
		let curr_size = self.get_size(superblock);
		if off > curr_size {
			return Err(errno::EINVAL);
		}

		let blk_size = superblock.get_block_size();
		let mut blk_buff = malloc::Alloc::<u8>::new_default(blk_size as usize)?;

		let mut i = 0;
		while i < buff.len() {
			let blk_off = (off + i as u64) / blk_size as u64;
			let blk_inner_off = ((off + i as u64) % blk_size as u64) as usize;
			let blk_off = {
				if let Some(blk_off) = self.get_content_block_off(blk_off as _, superblock, io)? {
					blk_off
				} else {
					self.alloc_content_block(blk_off as u32, superblock, io)?
				}
			};
			read_block(blk_off as _, superblock, io, blk_buff.get_slice_mut())?;

			let len = min(buff.len() - i, (blk_size - blk_inner_off as u32) as usize);
			unsafe { // Safe because staying in range
				copy_nonoverlapping(&buff[i] as *const u8,
					&mut blk_buff.get_slice_mut()[blk_inner_off] as *mut u8,
					len);
			}
			write_block(blk_off as _, superblock, io, blk_buff.get_slice_mut())?;

			i += len;
		}

		let new_size = max(off + buff.len() as u64, curr_size);
		self.set_size(superblock, new_size);
		Ok(())
	}

	/// Reads the directory entry at offset `off` and returns it.
	/// `superblock` is the filesystem's superblock.
	/// `io` is the I/O interface.
	/// `off` is the offset of the directory entry.
	/// If the file is not a directory, the behaviour is undefined.
	fn read_dirent(&self, superblock: &Superblock, io: &mut dyn IO, off: u64)
		-> Result<Box<DirectoryEntry>, Errno> {
		let mut buff: [u8; 8] = [0; 8];
		self.read_content(off as _, &mut buff, superblock, io)?;
		let entry = unsafe {
			DirectoryEntry::from(&buff)?
		};

		let mut buff = malloc::Alloc::<u8>::new_default(entry.get_total_size() as _)?;
		self.read_content(off as _, buff.get_slice_mut(), superblock, io)?;

		unsafe {
			DirectoryEntry::from(buff.get_slice())
		}
	}

	/// Writes the directory entry at offset `off`.
	/// `superblock` is the filesystem's superblock.
	/// `io` is the I/O interface.
	/// `off` is the offset of the directory entry.
	/// If the file is not a directory, the behaviour is undefined.
	fn write_dirent(&mut self, superblock: &Superblock, io: &mut dyn IO, entry: &DirectoryEntry,
		off: u64) -> Result<(), Errno> {
		let buff = unsafe {
			slice::from_raw_parts(entry as *const _ as *const u8, entry.get_total_size() as _)
		};

		self.write_content(off, buff, superblock, io)?;
		Ok(())
	}

	/// Iterates over directory entries and calls the given function `f` for each.
	/// The function takes the offset of the entry in the inode and the entry itself.
	/// Free entries are also included.
	/// `f` returns a boolean telling whether the iteration may continue.
	/// `superblock` is the filesystem's superblock.
	/// `io` is the I/O interface.
	/// If the file is not a directory, the behaviour is undefined.
	pub fn foreach_directory_entry<F: FnMut(u64, Box<DirectoryEntry>) -> bool>(&self, mut f: F,
		superblock: &Superblock, io: &mut dyn IO) -> Result<(), Errno> {
		debug_assert_eq!(self.get_type(), FileType::Directory);

		let blk_size = superblock.get_block_size();
		let mut buff = malloc::Alloc::<u8>::new_default(blk_size as usize)?;

		let size = self.get_size(superblock);
		let mut i = 0;
		while i < size {
			let len = min((size - i) as usize, blk_size as usize);
			self.read_content(i, &mut buff.get_slice_mut()[..len], superblock, io)?;

			let mut j = 0;
			while j < len {
				// Safe because the data is block-aligned and an entry cannot be larger than the
				// size of a block
				let entry = unsafe {
					DirectoryEntry::from(&buff.get_slice()[j..len])?
				};
				let total_size = entry.get_total_size() as usize;
				debug_assert!(total_size > 0);

				if !f(i + j as u64, entry) {
					return Ok(());
				}

				j += total_size;
			}

			i += blk_size as u64;
		}

		Ok(())
	}

	/// Returns the directory entry with the given name `name`.
	/// `superblock` is the filesystem's superblock.
	/// `io` is the I/O interface.
	/// If the entry doesn't exist, the function returns None.
	/// If the file is not a directory, the behaviour is undefined.
	pub fn get_directory_entry(&self, name: &[u8], superblock: &Superblock, io: &mut dyn IO)
		-> Result<Option<Box<DirectoryEntry>>, Errno> {
		let mut entry = None;

		// TODO If the binary tree feature is enabled, use it
		self.foreach_directory_entry(| _, e | {
			if !e.is_free() && e.get_name(superblock) == name {
				entry = Some(e);
				false
			} else {
				true
			}
		}, superblock, io)?;

		Ok(entry)
	}

	/// Looks for a free entry in the inode.
	/// `superblock` is the filesystem's superblock.
	/// `io` is the I/O interface.
	/// `min_size` is the minimum size of the entry in bytes.
	/// If the function finds an entry, it returns its offset. Else, the function returns None.
	fn get_free_entry(&self, superblock: &Superblock, io: &mut dyn IO, min_size: u16)
		-> Result<Option<u64>, Errno> {
		let mut off_option = None;

		self.foreach_directory_entry(| off, e | {
			if e.is_free() && e.get_total_size() >= min_size {
				off_option = Some(off);
				false
			} else {
				true
			}
		}, superblock, io)?;

		Ok(off_option)
	}

	/// Adds a new entry to the current directory.
	/// `superblock` is the filesystem's superblock.
	/// `io` is the I/O interface.
	/// `entry_inode` is the inode of the entry.
	/// `name` is the name of the entry.
	/// `file_type` is the type of the entry.
	/// If the block allocation fails or if the entry name is already used, the function returns an
	/// error.
	/// If the file is not a directory, the behaviour is undefined.
	pub fn add_dirent(&mut self, superblock: &Superblock, io: &mut dyn IO,
		entry_inode: u32, name: &String, file_type: FileType) -> Result<(), Errno> {
		let blk_size = superblock.get_block_size();
		let name_length = name.as_bytes().len() as u16;
		let entry_size = 8 + name_length;
		if entry_size as u32 > blk_size {
			return Err(errno::ENAMETOOLONG);
		}

		if let Some(free_entry_off) = self.get_free_entry(superblock, io, entry_size)? {
			let mut free_entry = self.read_dirent(superblock, io, free_entry_off)?;
			let split = free_entry.get_total_size() > entry_size + 8;

			if split {
				let new_entry = free_entry.split(entry_size)?;
				self.write_dirent(superblock, io, &new_entry, free_entry_off + entry_size as u64)?;
			}

			free_entry.set_inode(entry_inode);
			free_entry.set_name(superblock, name);
			free_entry.set_type(superblock, file_type);
			self.write_dirent(superblock, io, &free_entry, free_entry_off)
		} else {
			let entry = DirectoryEntry::new(superblock, entry_inode, blk_size as _, file_type, name)?;
			self.write_dirent(superblock, io, &entry, self.get_size(superblock))
		}
	}

	// TODO remove_dirent

	// TODO get_link_path

	/// Returns the device major and minor numbers associated with the device.
	/// If the file is not a device file, the behaviour is undefined.
	pub fn get_device(&self) -> (u8, u8) {
		debug_assert!(self.get_type() == FileType::BlockDevice
			|| self.get_type() == FileType::CharDevice);

		let dev = self.direct_block_ptrs[0];
		(((dev >> 8) & 0xff) as u8, (dev & 0xff) as u8)
	}

	/// Sets the device major and minor numbers associated with the device.
	/// `major` is the major number.
	/// `minor` is the minor number.
	/// If the file is not a device file, the behaviour is undefined.
	pub fn set_device(&mut self, major: u8, minor: u8) {
		debug_assert!(self.get_type() == FileType::BlockDevice
			|| self.get_type() == FileType::CharDevice);

		self.direct_block_ptrs[0] = ((major as u32) << 8) | (minor as u32);
	}

	/// Writes the inode on the device.
	pub fn write(&self, i: u32, superblock: &Superblock, io: &mut dyn IO) -> Result<(), Errno> {
		let off = Self::get_disk_offset(i, superblock, io)?;
		write(self, off, io)
	}
}
