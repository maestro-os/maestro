#include <disk/partition/partition.h>
#include <memory/memory.h>

static cache_t *partitions_cache = NULL, *mbr_partitions_cache = NULL;

// TODO Spinlock on each disk?

void partition_init(void)
{
	if(!(partitions_cache = cache_create("partitions",
		sizeof(partition_t), 32, bzero, NULL)))
		PANIC("Failed to initialize partitions manager!", 0);
	if(!(mbr_partitions_cache = cache_create("mbr_partitions",
		sizeof(mbr_partition_t), 32, bzero, NULL)))
		PANIC("Failed to initialize partitions manager!", 0);
}

static void partition_table_create(disk_t *disk)
{
	// TODO Create GPT instead
	mbr_create(disk);
}

static void insert_partition(disk_t *disk, partition_t *partition)
{
	partition_t *tmp;

	if((tmp = disk->partitions))
	{
		while(tmp->next)
			tmp = tmp->next;
		tmp->next = partition;
	}
	else
		disk->partitions = partition;
}

static partition_t *mbr_to_partition(disk_t *disk, const partition_id_t id,
	const mbr_entry_t entry)
{
	mbr_partition_t *p;
	partition_t *partition;

	if(!disk || !(p = cache_alloc(mbr_partitions_cache)))
		return NULL;
	mbr_etop(entry, p);
	if(p->partition_type == 0x0 || !(partition = cache_alloc(partitions_cache)))
	{
		cache_free(mbr_partitions_cache, p);
		return NULL;
	}
	partition->disk = disk;
	partition->id = id;
	partition->type = p->partition_type;
	partition->partition_struct = p;
	partition->start_lba = p->start_lba;
	partition->sectors = p->sectors;
	insert_partition(disk, partition);
	return partition;
}

// TODO embr_to_partition
// TODO gpt_to_partition

void partition_read_table(disk_t *disk)
{
	char buff[ATA_SECTOR_SIZE];
	mbr_t *mbr;
	size_t i;

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
	// TODO Handle extended MBR
	for(i = 0; i < MBR_ENTRIES_COUNT; ++i)
		mbr_to_partition(disk, i, mbr->entries[i]); // TODO Check for errno
}

// TODO Remove
#include <tty/tty.h>

partition_t *partition_create(disk_t *dev, const partition_type_t type)
{
	char buff[ATA_SECTOR_SIZE];
	mbr_t *mbr;
	partition_t *partition;
	mbr_partition_t *mbr_partition;

	if(!dev)
		return NULL;
	disk_select_disk(dev);
	if(disk_read(0, buff, 1) < 0)
		goto fail;
	mbr = (void *) buff + MBR_PARTITION_TABLE_OFFSET;
	if(mbr->boot_signature != MBR_SIGNATURE)
	{
		partition_table_create(dev);
		if(disk_read(0, buff, 1) < 0)
			goto fail;
	}
	// TODO Check for GPT
	if(!(partition = cache_alloc(partitions_cache))
		|| !(mbr_partition = cache_alloc(mbr_partitions_cache)))
		goto fail;
	partition->id = 0; // TODO
	partition->type = type;
	partition->partition_struct = mbr_partition;
	partition->start_lba = 0; // TODO
	partition->sectors = 0; // TODO
	mbr_partition->partition_type = type;
	mbr_partition->start_lba = 0; // TODO
	mbr_partition->sectors = 0; // TODO
	mbr_ptoe(mbr_partition, mbr->entries + partition->id);
	if(disk_write(0, buff, 1) < 0)
		goto fail;
	insert_partition(dev, partition);
	return partition;

fail:
	cache_free(partitions_cache, partition);
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
