#include <disk/ext2/ext2.h>

static int ext2_needs_consistency_check(const ext2_superblock_t *superblock)
{
	return (superblock->mounts_since_last_check
		>= superblock->max_mounts_between_checks)
			|| (time_get() >= superblock->last_check_time
				+ superblock->check_interval_time);
}

static int ext2_check_superblock(const ext2_superblock_t *superblock)
{
	int readonly = 0;

	if(superblock->signature != EXT2_SIGNATURE)
		return MOUNT_STATE_ERROR;
	if(superblock->major_version >= 1)
	{
		// TODO Handle extended superblock
	}
	if(superblock->state != EXT2_STATE_CLEAN)
	{
		if(superblock->error_handling_method == EXT2_ERROR_HANDLING_READONLY)
			readonly = 1;
		else if(superblock->error_handling_method == EXT2_ERROR_HANDLING_PANIC)
			PANIC("Ext2 filesystem has errors", 0);
	}
	if(superblock->superuser_reserved_blocks >= superblock->total_blocks)
		return MOUNT_STATE_ERROR;
	// TODO Check block size (must be larger than one sector)
	if(ext2_needs_consistency_check(superblock))
		return MOUNT_STATE_NEEDS_CHECK;
	return (readonly ? MOUNT_STATE_READONLY : MOUNT_STATE_MOUNTED);
}

int ext2_mount(void)
{
	char buff[ATA_SECTOR_SIZE * EXT2_SUPERBLOCK_SECTORS];
	ext2_superblock_t *superblock;
	int mount_state;

	if((disk_read(EXT2_BEGIN_SECTOR, buff, EXT2_SUPERBLOCK_SECTORS)) < 0)
		return MOUNT_STATE_DISK_ERROR;
	superblock = (void *) buff;
	mount_state = ext2_check_superblock(superblock);
	++superblock->mounts_since_last_check;
	superblock->last_mount_time = time_get();
	disk_write(EXT2_BEGIN_SECTOR, buff, 1);
	return mount_state;
}

int ext2_consistency_check(void)
{
	// TODO
	return 0;
}
