#include <disk/partition/partition.h>

partition_id_t partition_create(ata_device_t *dev, const uint8_t partition_type)
{
	if(!dev)
		return 0;
	// TODO If no partition table, create one (GPT)
	// TODO
	(void) partition_type;
	return 0;
}

void parition_move(ata_device_t *dev,
	const partition_id_t id, const size_t lba)
{
	if(!dev)
		return;
	// TODO
	(void) id;
	(void) lba;
}

void parition_resize(ata_device_t *dev,
	const partition_id_t id, const size_t sectors)
{
	if(!dev)
		return;
	// TODO
	(void) id;
	(void) sectors;
}

void partition_remove(ata_device_t *dev, const partition_id_t id)
{
	if(!dev)
		return;
	// TODO
	(void) id;
}
