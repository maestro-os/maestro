#ifndef DISK_H
# define DISK_H

# include <kernel.h>
# include <disk/ata/ata.h>
# include <disk/partition/partition.h>

void disk_select_partition(ata_device_t *dev, mbr_partition_t *partition);
int disk_read(size_t sector, char *buff, size_t sectors_count);
int disk_write(size_t sector, const char *buff, size_t sectors_count);

#endif
