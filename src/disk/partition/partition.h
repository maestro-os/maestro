#ifndef PARTITION_H
# define PARTITION_H

# include <kernel.h>
# include <disk/disk.h>

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

typedef struct disk disk_t;
typedef uint16_t partition_id_t;
typedef uint8_t partition_type_t;

typedef struct partition
{
	struct partition *next;
	disk_t *disk;

	partition_id_t id;
	partition_type_t type;
	void *partition_struct;

	size_t start_lba;
	size_t sectors;
} partition_t;

// TODO Implement Extended Partitions
// TODO Implement GPT

void mbr_etop(const mbr_entry_t entry, mbr_partition_t *partition);
void mbr_ptoe(mbr_partition_t *partition, mbr_entry_t entry);

void mbr_init(disk_t *dev);
int mbr_read(disk_t *dev, size_t lba, mbr_partition_t *partitions);
int mbr_write(disk_t *dev, size_t lba, mbr_t *mbr);

void partition_read_table(disk_t *disk);
partition_t *partition_create(disk_t *dev, partition_type_t partition_type);
partition_t *partition_get(disk_t *dev, partition_id_t id);
void parition_move(partition_t *partition, size_t lba);
void parition_resize(partition_t *partition, size_t sectors);
void partition_remove(partition_t *partition);

#endif
