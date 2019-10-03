#include <disk/disk.h>
#include <libc/errno.h>
#include <libc/math.h>

static ata_device_t *current_device = NULL;
static uint32_t start_lba = 0, end_lba = 0;

void disk_select_partition(ata_device_t *dev, mbr_partition_t *partition)
{
	if(!dev || !partition)
	{
		current_device = NULL;
		return;
	}
	current_device = dev;
	start_lba = partition->start_lba;
	end_lba = start_lba + partition->sectors;
	if(end_lba <= start_lba)
		current_device = NULL;
}

int disk_read(const size_t sector, char *buff, const size_t sectors_count)
{
	size_t i;

	if(!buff || sectors_count == 0
		|| !current_device || sector >= end_lba - start_lba)
		return -1;
	for(i = 0; i < sectors_count; i += 0xff)
		ata_read(current_device, start_lba + sector + i,
			buff + (i * ATA_SECTOR_SIZE), min(sectors_count - i, 0xff));
	return (errno ? -1 : 0);
}

int disk_write(const size_t sector, const char *buff,
	const size_t sectors_count)
{
	size_t i;

	if(!buff || sectors_count == 0
		|| !current_device || sector >= end_lba - start_lba)
		return -1;
	for(i = 0; i < sectors_count; i += 0xff)
		ata_write(current_device, start_lba + sector + i,
			buff + (i * ATA_SECTOR_SIZE), min(sectors_count - i, 0xff));
	return (errno ? -1 : 0);
}
