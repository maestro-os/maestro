#include <disk/disk.h>
#include <libc/errno.h>
#include <libc/math.h>

static void *current_device = NULL;
static disk_read_func_t read_func = NULL;
static disk_write_func_t write_func = NULL;

static uint32_t start_lba = 0, end_lba = 0;

static spinlock_t spinlock = 0;

static void disk_set_io_funcs(const disk_type_t type)
{
	switch(type)
	{
		case DISK_TYPE_ATA:
		{
			read_func = (disk_read_func_t) ata_read;
			write_func = (disk_write_func_t) ata_write;
			break;
		}

		// TODO

		default:
		{
			read_func = NULL;
			write_func = NULL;
			break;
		}
	}
}

void disk_select_disk(const disk_t *disk)
{
	spin_lock(&spinlock);
	if(!disk || !disk->disk_struct)
	{
		current_device = NULL;
		goto end;
	}
	current_device = disk->disk_struct;
	disk_set_io_funcs(disk->type);
	start_lba = 0;
	if((end_lba = disk->sectors) == 0)
		current_device = NULL;

end:
	spin_unlock(&spinlock);
}

void disk_select_partition(const partition_t *partition)
{
	spin_lock(&spinlock);
	if(!partition || !partition->disk || !partition->disk->disk_struct)
	{
		current_device = NULL;
		goto end;
	}
	current_device = partition->disk->disk_struct;
	disk_set_io_funcs(partition->disk->type);
	start_lba = partition->start_lba;
	end_lba = start_lba + partition->sectors;
	if(end_lba <= start_lba)
		current_device = NULL;

end:
	spin_unlock(&spinlock);
}

int disk_read(const size_t sector, char *buff, const size_t sectors_count)
{
	size_t i;

	if(!buff || sectors_count == 0 || sector >= end_lba - start_lba)
		return -1;
	if(!current_device || !read_func)
		return -1;
	spin_lock(&spinlock);
	for(i = 0; i < sectors_count; i += 0xff)
		read_func(current_device, start_lba + sector + i,
			buff + (i * ATA_SECTOR_SIZE), min(sectors_count - i, 0xff));
	spin_unlock(&spinlock);
	return (errno ? -1 : 0);
}

int disk_write(const size_t sector, const char *buff,
	const size_t sectors_count)
{
	size_t i;

	if(!buff || sectors_count == 0 || sector >= end_lba - start_lba)
		return -1;
	if(!current_device || !write_func)
		return -1;
	spin_lock(&spinlock);
	for(i = 0; i < sectors_count; i += 0xff)
		write_func(current_device, start_lba + sector + i,
			buff + (i * ATA_SECTOR_SIZE), min(sectors_count - i, 0xff));
	spin_unlock(&spinlock);
	return (errno ? -1 : 0);
}
