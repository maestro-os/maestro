#include <disk/partition/partition.h>

// TODO Spinlock on disk?

void mbr_create(disk_t *dev)
{
	char buff[ATA_SECTOR_SIZE];
	mbr_t *mbr;

	if(!dev)
		return;
	disk_select_disk(dev);
	if(disk_read(0, buff, 1) < 0)
		return; // TODO Panic?
	mbr = (void *) buff + MBR_PARTITION_TABLE_OFFSET;
	bzero(mbr + 6, sizeof(mbr) - 8);
	mbr->boot_signature = MBR_SIGNATURE;
	disk_write(0, buff, 1); // TODO Protect
}

void mbr_etop(const mbr_entry_t entry, mbr_partition_t *partition)
{
	if(!entry || !partition)
		return;
	bzero(partition, sizeof(mbr_partition_t));
	partition->attrs = entry[0];
	memcpy(((void *) &partition->chs_addr) + 1, entry + 1, 3);
	partition->partition_type = entry[4];
	memcpy(((void *) &partition->chs_addr_last) + 1, entry + 5, 3);
	partition->start_lba = *(uint32_t *) (entry + 8);
	partition->sectors = *(uint32_t *) (entry + 12);
}

void mbr_ptoe(mbr_partition_t *partition, void *entry)
{
	char *e;

	if(!partition || !entry)
		return;
	e = entry;
	e[0] = partition->attrs;
	memcpy(e + 1, ((void *) &partition->chs_addr) + 1, 3);
	e[4] = partition->partition_type;
	memcpy(e + 5, ((void *) &partition->chs_addr_last) + 1, 3);
	*(uint32_t *) (e + 8) = partition->start_lba;
	*(uint32_t *) (e + 12) = partition->sectors;
}
