#ifndef DISK_H
# define DISK_H

# include <kernel.h>
# include <disk/ata/ata.h>
# include <disk/partition/partition.h>

// TODO Check for fail on every I/O

typedef enum
{
	DISK_TYPE_UNKNOWN,
	DISK_TYPE_ATA
	// TODO
} disk_type_t;

typedef struct partition partition_t;

typedef struct disk
{
	struct disk *next;

	disk_type_t type;
	void *disk_struct;

	size_t sectors;
	size_t sector_size;

	partition_t *partitions;
} disk_t;

typedef int (*disk_read_func_t)(void *, size_t, void *, size_t);
typedef int (*disk_write_func_t)(void *, size_t, const void *, size_t);

extern disk_t *disks;

void disk_init(void);
void disk_select_disk(const disk_t *disk);
void disk_select_partition(const partition_t *partition);
int disk_read(size_t sector, char *buff, size_t sectors_count);
int disk_write(size_t sector, const char *buff, size_t sectors_count);

#endif
