#include <disk/partition/partition.h>

// TODO Spinlock on each disk?

static void partition_table_create(disk_t *disk)
{
	// TODO Create GPT instead
	mbr_init(disk);
}

void partition_read_table(disk_t *disk)
{
	char buff[ATA_SECTOR_SIZE];
	mbr_t *mbr;

	if(!disk)
		return;
	disk_select_disk(disk);
	if(disk_read(0, buff, 1) < 0)
	{
		// TODO Error
	}
	mbr = (void *) buff + MBR_PARTITION_TABLE_OFFSET;
	if(mbr->boot_signature != MBR_SIGNATURE)
	{
		partition_table_create(disk);
		return;
	}
	// TODO Check for GPT
}

partition_t *partition_create(disk_t *dev,
	const partition_type_t partition_type)
{
	char buff[ATA_SECTOR_SIZE];
	mbr_t *mbr;

	if(!dev)
		return NULL;
	disk_select_disk(disk);
	if(disk_read(0, buff, 1) < 0)
	{
		// TODO Error
	}
	mbr = (void *) buff + MBR_PARTITION_TABLE_OFFSET;
	if(mbr->boot_signature != MBR_SIGNATURE)
	{
		partition_table_create(disk);
		if(disk_read(0, buff, 1) < 0)
		{
			// TODO Error
		}
	}
	// TODO Check for GPT
	// TODO Create partition
	(void) partition_type;
	return NULL;
}

partition_t *partition_get(disk_t *dev, const partition_id_t id)
{
	partition_t *p;

	if(!dev)
		return NULL;
	p = dev->partitions;
	while(p)
	{
		if(p->id == id)
			return p;
		p = p->next;
	}
	return NULL;
}

// TODO Check if overlapping
void parition_move(partition_t *partition, const size_t lba)
{
	if(!partition ||!partition->disk)
		return;
	// TODO
	(void) lba;
}

// TODO Check if overlapping
void parition_resize(partition_t *partition, const size_t sectors)
{
	if(!partition ||!partition->disk)
		return;
	// TODO
	(void) sectors;
}

void partition_remove(partition_t *partition)
{
	if(!partition || !partition->disk)
		return;
	// TODO
}
