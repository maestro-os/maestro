#ifndef EXT2_H
# define EXT2_H

# include <kernel.h>
# include <disk/disk.h>
# include <cmos/cmos.h>

# define EXT2_PARTITION_TYPE		0x83
# define EXT2_BEGIN_SECTOR			2
# define EXT2_SUPERBLOCK_SECTORS	2

# define EXT2_SIGNATURE	0xef53

# define EXT2_STATE_CLEAN	1
# define EXT2_STATE_ERROR	2

# define EXT2_ERROR_HANDLING_CONTINUE	1
# define EXT2_ERROR_HANDLING_READONLY	2
# define EXT2_ERROR_HANDLING_PANIC		3

# define EXT2_OS_ID_LINUX		0
# define EXT2_OS_ID_GNU_HURD	1
# define EXT2_OS_ID_MASIX		2
# define EXT2_OS_ID_FREEBSD		3
# define EXT2_OS_ID_OTHER		4

# define EXT2_INODE_TYPE_FIFO			0x1000
# define EXT2_INODE_TYPE_CHAR_DEVICE	0x2000
# define EXT2_INODE_TYPE_DIRECTORY		0x4000
# define EXT2_INODE_TYPE_BLOCK_DEVICE	0x6000
# define EXT2_INODE_TYPE_REGULAR_FILE	0x8000
# define EXT2_INODE_TYPE_SYMBOLIC_LINK	0xa000
# define EXT2_INODE_TYPE_UNIX_SOCKET	0xc000

# define EXT2_PERMISSION_XOTH			0x1
# define EXT2_PERMISSION_WOTH			0x2
# define EXT2_PERMISSION_ROTH			0x4
# define EXT2_PERMISSION_XGRP			0x8
# define EXT2_PERMISSION_WGRP			0x10
# define EXT2_PERMISSION_RGRP			0x20
# define EXT2_PERMISSION_XUSR			0x40
# define EXT2_PERMISSION_WUSR			0x80
# define EXT2_PERMISSION_RUSR			0x100
# define EXT2_PERMISSION_STICKY			0x200
# define EXT2_PERMISSION_SET_GROUP_ID	0x400
# define EXT2_PERMISSION_SET_USER_ID	0x800

# define EXT2_FLAG_SECURE_DELETION	0x00001
# define EXT2_FLAG_KEEP_COPY		0x00002
# define EXT2_FLAG_FILE_COMPRESSION	0x00004
# define EXT2_FLAG_SYNC_UPDATE		0x00008
# define EXT2_FLAG_IMMUTABLE		0x00010
# define EXT2_FLAG_APPEND_ONLY		0x00020
# define EXT2_FLAG_NODUMP			0x00040
# define EXT2_FLAG_NO_TIME_UPDATE	0x00080
# define EXT2_FLAG_HASH_INDEXED_DIR	0x10000
# define EXT2_FLAG_AFS_DIRECTORY	0x20000
# define EXT2_FLAG_JOURNAL_FILE		0x40000

# define EXT2_DIRECTORY_ENTRY_TYPE_UNKNOWN			0
# define EXT2_DIRECTORY_ENTRY_TYPE_REGULAR			1
# define EXT2_DIRECTORY_ENTRY_TYPE_DIRECTORY		2
# define EXT2_DIRECTORY_ENTRY_TYPE_CHAR_DEVICE		3
# define EXT2_DIRECTORY_ENTRY_TYPE_BLOCK_DEVICE		4
# define EXT2_DIRECTORY_ENTRY_TYPE_FIFO				5
# define EXT2_DIRECTORY_ENTRY_TYPE_SOCKET			6
# define EXT2_DIRECTORY_ENTRY_TYPE_SYMBOLIC_LINK	7

# define MOUNT_STATE_ERROR			0
# define MOUNT_STATE_DISK_ERROR		1
# define MOUNT_STATE_MOUNTED		2
# define MOUNT_STATE_READONLY		3
# define MOUNT_STATE_NEEDS_CHECK	4

__attribute__((packed))
struct ext2_superblock
{
	uint32_t total_inodes;
	uint32_t total_blocks;
	uint32_t superuser_reserved_blocks;
	uint32_t unallocated_blocks;
	uint32_t unallocated_inodes;
	uint32_t superblock_number;
	uint32_t block_size;
	uint32_t fragment_size;
	uint32_t blocks_per_group;
	uint32_t fragments_per_group;
	uint32_t inodes_per_group;
	uint32_t last_mount_time;
	uint32_t last_write_time;
	uint16_t mounts_since_last_check;
	uint16_t max_mounts_between_checks;
	uint16_t signature;
	uint16_t state;
	uint16_t error_handling_method;
	uint16_t minor_version;
	uint32_t last_check_time;
	uint32_t check_interval_time;
	uint32_t os_id;
	uint32_t major_version;
	uint16_t superuser;
	uint16_t supergroup;
};

__attribute__((packed))
struct ext2_extended_superblock
{
	struct ext2_superblock base;
	// TODO
};

__attribute__((packed))
struct ext2_block_group_descriptor
{
	uint32_t block_usage_bitmap_addr;
	uint32_t inode_usage_bitmap_addr;
	uint32_t inode_table_start_block;
	uint16_t unallocated_blocks;
	uint16_t unallocated_inodes;
	uint16_t directories;
};

__attribute__((packed))
struct ext2_inode
{
	uint16_t type_permission;
	uint16_t user_id;
	uint32_t size_low;
	uint32_t last_access;
	uint32_t creation_time;
	uint32_t last_modification;
	uint32_t delete_time;
	uint16_t group_id;
	uint16_t hard_links;
	uint32_t disk_sectors;
	uint32_t flags;
	uint32_t os_specific_1;
	uint32_t direct_block_pointers[12];
	uint32_t singly_indirect_block;
	uint32_t doubly_indirect_block;
	uint32_t triply_indirect_block;
	uint32_t generation_number;
	uint32_t extended_attribute_block;
	uint32_t size_upper;
	uint32_t fragment_addr;
	uint32_t os_specific_2[3];
};

__attribute__((packed))
struct ext2_directory_entry
{
	uint32_t inode;
	uint16_t total_size;
	uint8_t name_length_low;
	uint8_t name_length_high;
	char name[0];
};

typedef struct ext2_superblock ext2_superblock_t;
typedef struct ext2_extended_superblock ext2_extended_superblock_t;
typedef struct ext2_block_group_descriptor ext2_block_group_descriptor_t;
typedef struct ext2_inode ext2_inode_t;
typedef struct ext2_directory_entry ext2_directory_entry_t;

void ext2_create(void);
int ext2_mount(void);
int ext2_consistency_check(void);
// TODO

#endif
