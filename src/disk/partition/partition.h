#ifndef PARTITION_H
# define PARTITION_H

# include <kernel.h>
# include <disk/ata/ata.h>

# define MBR_PARTITION_TABLE_OFFSET	0x1b8
# define MBR_ENTRIES_COUNT			4

typedef char mbr_entry_t[16];

__attribute__((packed))
struct mbr
{
	uint32_t signature;
	uint16_t reserved;
	mbr_entry_t entries[MBR_ENTRIES_COUNT];
	uint16_t boot_signature;
};

typedef struct mbr mbr_t;

typedef struct
{
	uint8_t attrs;
	uint32_t chs_addr;
	uint8_t partition_type;
	uint32_t chs_addr_last;
	uint32_t start_lba;
	uint32_t sectors;
} mbr_partition_t;

void mbr_read(ata_device_t *dev, size_t lba, mbr_partition_t *partitions);

#endif
