#include <disk/partition/partition.h>

static void mbr_entry_convert(const mbr_entry_t entry,
	mbr_partition_t *partition)
{
	bzero(partition, sizeof(mbr_partition_t));
	partition->attrs = entry[0];
	memcpy(&partition->chs_addr, entry + 1, 3);
	partition->partition_type = entry[4];
	memcpy(&partition->chs_addr_last, entry + 5, 3);
	partition->start_lba = *(uint32_t *) (entry + 8);
	partition->sectors = *(uint32_t *) (entry + 12);
}

void mbr_read(ata_device_t *dev, size_t lba, mbr_partition_t *partitions)
{
	char buff[ATA_SECTOR_SIZE];
	mbr_t mbr;
	size_t i;

	if(!dev || !partitions)
		return;
	// TODO Use default device
	ata_read(dev, 0, lba, buff, 1);
	memcpy(&mbr, buff + MBR_PARTITION_TABLE_OFFSET, sizeof(mbr_t));
	for(i = 0; i < MBR_ENTRIES_COUNT; ++i)
		mbr_entry_convert(mbr.entries[i], partitions + i);
}
