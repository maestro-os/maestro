#include <disk/partition/partition.h>

// TODO Spinlock on each disk?

partition_t *partition_create(disk_t *dev,
	const partition_type_t partition_type)
{
	if(!dev)
		return 0;
	// TODO If no partition table, create one (GPT)
	// TODO
	(void) partition_type;
	return 0;
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
