//! The ext2 filesystem is a classical filesystem used in Unix systems.
//! It is nowdays obsolete and has been replaced by ext3 and ext4.

use crate::device::DeviceHandle;
use crate::errno::Errno;
use crate::errno;
use crate::file::File;
use crate::file::INode;
use crate::file::filesystem::Filesystem;
use crate::file::filesystem::FilesystemType;
use crate::file::path::Path;
use crate::util::boxed::Box;

/// The offset of the superblock from the beginning of the device.
const SUPERBLOCK_OFFSET: usize = 1024;
/// The filesystem's signature.
const EXT2_SIGNATURE: u16 = 0xef53;

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
const OPTIONAL_FEATURE_DIRECTORY_PREALLOCATION: u16 = 0x1;
/// Optional feature: AFS server
const OPTIONAL_FEATURE_AFS: u16 = 0x2;
/// Optional feature: Journal
const OPTIONAL_FEATURE_JOURNAL: u16 = 0x4;
/// Optional feature: Inodes have extended attributes
const OPTIONAL_FEATURE_INODE_EXTENDED: u16 = 0x8;
/// Optional feature: Filesystem can resize itself for larger partitions
const OPTIONAL_FEATURE_RESIZE: u16 = 0x10;
/// Optional feature: Directories use hash index
const OPTIONAL_FEATURE_HASH_INDEX: u16 = 0x20;

/// Required feature: Compression
const REQUIRED_FEATURE_COMPRESSION: u16 = 0x1;
/// Required feature: Directory entries have a type field
const REQUIRED_FEATURE_DIRECTORY_TYPE: u16 = 0x2;
/// Required feature: Filesystem needs to replay its journal
const REQUIRED_FEATURE_JOURNAL_REPLAY: u16 = 0x4;
/// Required feature: Filesystem uses a journal device
const REQUIRED_FEATURE_JOURNAL_DEVIXE: u16 = 0x8;

/// Write-required feature: Sparse superblocks and group descriptor tables
const WRITE_REQUIRED_SPARSE_SUPERBLOCKS: u16 = 0x1;
/// Write-required feature: Filesystem uses a 64-bit file size
const WRITE_REQUIRED_64_BITS: u16 = 0x2;
/// Directory contents are stored in the form of a Binary Tree.
const WRITE_REQUIRED_DIRECTORY_BINARY_TREE: u16 = 0x4;

/// INode type: FIFO
const INODE_TYPE_FIFO: u16 = 0x1000;
/// INode type: Char device
const INODE_TYPE_CHAR_DEVICE: u16 = 0x2000;
/// INode type: Directory
const INODE_TYPE_DIRECTORY: u16 = 0x4000;
/// INode type: Block device
const INODE_TYPE_BLOCK_DEVICE: u16 = 0x6000;
/// INode type: Regular file
const INODE_TYPE_REGULAR: u16 = 0x8000;
/// INode type: Symbolic link
const INODE_TYPE_SYMLINK: u16 = 0xa000;
/// INode type: Socket
const INODE_TYPE_SOCKET: u16 = 0xc000;

/// User: Read, Write and Execute.
const INODE_PERMISSION_IRWXU: u16 = 00700;
/// User: Read.
const INODE_PERMISSION_IRUSR: u16 = 00400;
/// User: Write.
const INODE_PERMISSION_IWUSR: u16 = 00200;
/// User: Execute.
const INODE_PERMISSION_IXUSR: u16 = 00100;
/// Group: Read, Write and Execute.
const INODE_PERMISSION_IRWXG: u16 = 00070;
/// Group: Read.
const INODE_PERMISSION_IRGRP: u16 = 00040;
/// Group: Write.
const INODE_PERMISSION_IWGRP: u16 = 00020;
/// Group: Execute.
const INODE_PERMISSION_IXGRP: u16 = 00010;
/// Other: Read, Write and Execute.
const INODE_PERMISSION_IRWXO: u16 = 00007;
/// Other: Read.
const INODE_PERMISSION_IROTH: u16 = 00004;
/// Other: Write.
const INODE_PERMISSION_IWOTH: u16 = 00002;
/// Other: Execute.
const INODE_PERMISSION_IXOTH: u16 = 00001;
/// Setuid.
const INODE_PERMISSION_ISUID: u16 = 04000;
/// Setgid.
const INODE_PERMISSION_ISGID: u16 = 02000;
/// Sticky bit.
const INODE_PERMISSION_ISVTX: u16 = 01000;

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
const ROOT_DIRECTORY_INODE: u32 = 2;

/// Directory entry type indicator: Unknown
const TYPE_INDICATOR_UNKNOWN: u8 = 0;
/// Directory entry type indicator: Regular file
const TYPE_INDICATOR_REGULAR: u8 = 1;
/// Directory entry type indicator: Directory
const TYPE_INDICATOR_DIRECTORY: u8 = 2;
/// Directory entry type indicator: Char device
const TYPE_INDICATOR_CHAR_DEVICE: u8 = 3;
/// Directory entry type indicator: Block device
const TYPE_INDICATOR_BLOCK_DEVICE: u8 = 4;
/// Directory entry type indicator: FIFO
const TYPE_INDICATOR_FIFO: u8 = 5;
/// Directory entry type indicator: Socket
const TYPE_INDICATOR_SOCKET: u8 = 6;
/// Directory entry type indicator: Symbolic link
const TYPE_INDICATOR_SYMLINK: u8 = 7;

/// The ext2 superblock structure.
#[repr(C, packed)]
struct Superblock {
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
	/// TODO doc
	block_size_mask_shift: u32,
	/// TODO doc
	fragment_size_mask_shift: u32,
	/// The number of blocks per block group.
	blocks_per_block_group: u32,
	/// The number of fragments per block group.
	fragments_per_block_group: u32,
	/// The number of inodes per block group.
	inodes_per_block_group: u32,
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
	superblock_block_group: u16,
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
	/// TODO doc
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

	// TODO Add padding?
}

/// TODO doc
#[repr(C, packed)]
struct BlockGroupDescriptor {
	/// The block address of the block usage bitmap.
	block_usage_bitmap_addr: u32,
	/// The block address of the inode usage bitmap.
	inode_usage_bitmap_addr: u32,
	/// Starting block address of inode table.
	inode_table_start_addr: u32,
	/// Number of unallocated blocks in group.
	unallocated_blocks_number: u16,
	/// Number of unallocated inodes in group.
	unallocated_inodes_number: u16,
	/// Number of directories in group.
	directories_number: u16,

	// TODO Add padding?
}

/// TODO doc
#[repr(C, packed)]
struct Ext2INode {
	/// Type and permissions.
	type_permissions: u16,
	/// User ID.
	uid: u16,
	/// Lower 32 bits of size in bytes.
	size_low: u32,
	/// Timestamp of the last modification of the metadata.
	ctime: u16,
	/// Timestamp of the last modification of the content.
	mtime: u16,
	/// Timestamp of the last access.
	atime: u16,
	/// Timestamp of the deletion.
	dtime: u16,
	/// Group ID.
	gid: u16,
	/// The number of hard links to this inode.
	hard_links_count: u16,
	/// The number of sectors used by this inode.
	used_sectors: u32,
	/// INode flags.
	flags: u32,
	/// OS-specific value.
	os_specific_0: u32,
	/// Direct block pointers.
	direct_block_ptrs: [u32; 12],
	/// TODO doc
	singly_indirect_block_ptr: u32,
	/// TODO doc
	doubly_indirect_block_ptr: u32,
	/// TODO doc
	triply_indirect_block_ptr: u32,
	/// Generation number.
	generation: u32,
	/// TODO doc
	extended_attributes_block: u32,
	/// Higher 32 bits of size in bytes.
	size_high: u32,
	/// Block address of fragment.
	fragment_addr: u32,
	/// OS-specific value.
	os_specific_1: u32,
}

/// TODO doc
struct DirectoryEntry {
	/// The inode associated with the entry.
	inode: u32,
	/// The total size of the entry.
	total_size: u16,
	/// Name length least-significant bits.
	name_length_lo: u8,
	/// Name length most-significant bits or type indicator (if enabled).
	name_length_hi: u8,
	/// The entry's name.
	name: [u8; 0],
}

/// Structure representing the ext2 filesystem type.
pub struct Ext2FsType {}

impl FilesystemType for Ext2FsType {
	fn get_name(&self) -> &str {
		"ext2"
	}

	fn detect(&self, _io: &mut dyn DeviceHandle) -> bool {
		// TODO

		false
	}

	fn new_filesystem(&self, _io: &mut dyn DeviceHandle) -> Result<Box<dyn Filesystem>, Errno> {
		// TODO
		Err(errno::ENOMEM)
	}
}

/// Structure representing a instance of the ext2 filesystem.
pub struct Ext2Fs {}

impl Ext2Fs {
	/// Creates a new instance.
	pub fn new() -> Self {
		Self {}
	}
}

impl Filesystem for Ext2Fs {
	fn get_name(&self) -> &str {
		"ext2"
	}

	fn load_file(&mut self, _io: &mut dyn DeviceHandle, _path: Path) -> Result<File, Errno> {
		// TODO

		Err(errno::ENOMEM)
	}

	fn read_node(&mut self, _io: &mut dyn DeviceHandle, _node: INode, _buf: &mut [u8])
		-> Result<(), Errno> {
		// TODO

		Err(errno::ENOMEM)
	}

	fn write_node(&mut self, _io: &mut dyn DeviceHandle, _node: INode, _buf: &mut [u8])
		-> Result<(), Errno> {
		// TODO

		Err(errno::ENOMEM)
	}
}
