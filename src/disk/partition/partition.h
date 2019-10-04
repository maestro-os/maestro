#ifndef PARTITION_H
# define PARTITION_H

# include <kernel.h>
# include <disk/ata/ata.h>

# define MBR_PARTITION_TABLE_OFFSET	0x1b8
# define MBR_ENTRIES_COUNT			4
# define MBR_SIGNATURE				0x55aa

# define GPT_SIGNATURE	"EFI PART"

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

// TODO Implement Extended Partitions
// TODO Implement GPT

typedef uint16_t partition_id_t;

void mbr_etop(const mbr_entry_t entry, mbr_partition_t *partition);
void mbr_ptoe(mbr_partition_t *partition, mbr_entry_t entry);

void mbr_init(mbr_t *mbr);
int mbr_read(ata_device_t *dev, size_t lba, mbr_partition_t *partitions);
int mbr_write(ata_device_t *dev, size_t lba, mbr_t *mbr);

partition_id_t partition_create(ata_device_t *dev, uint8_t parition_type);
// TODO Get a single parition or paritions list for a device
void parition_move(ata_device_t *dev, partition_id_t id, size_t lba);
void parition_resize(ata_device_t *dev, partition_id_t id, size_t sectors);
void partition_remove(ata_device_t *dev, partition_id_t id);

#endif
