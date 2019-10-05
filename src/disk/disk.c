#include <disk/disk.h>
#include <memory/memory.h>
#include <libc/errno.h>
#include <libc/math.h>

static cache_t *disks_cache, *partitions_cache;
disk_t *disks = NULL;

static void *current_device = NULL;
static disk_read_func_t read_func = NULL;
static disk_write_func_t write_func = NULL;

static uint32_t start_lba = 0, end_lba = 0;

static spinlock_t spinlock = 0;

// TODO Create a RAM cache? (might be less good for SATA)

static void insert_disk(disk_t *disk)
{
	disk_t *d;

	if((d = disks))
	{
		while(d->next)
			d = d->next;
		d->next = disk;
	}
	else
		disks = disk;
}

static void disk_new_ata(const uint16_t bus, const uint16_t ctrl)
{
	disk_t *disk;
	ata_device_t *d;

	if(!(disk = cache_alloc(disks_cache)))
		PANIC("Cannot allocate memory for hard disk", 0);
	disk->type = DISK_TYPE_ATA;
	disk->disk_struct = (d = ata_init_device(bus, ctrl));
	disk->sectors = d->sectors;
	disk->sector_size = ATA_SECTOR_SIZE;
	partition_read_table(disk);
	if(errno)
		PANIC("Cannot allocate memory for partition", 0);
	insert_disk(disk);
}

void disk_init(void)
{
	if(!(disks_cache = cache_create("disks", sizeof(disk_t), 32, bzero, NULL)))
		PANIC("Failed to initialize disks manager!", 0);
	if(!(partitions_cache = cache_create("partitions", sizeof(partition_t),
		32, bzero, NULL)))
		PANIC("Failed to initialize disks manager!", 0);
	// TODO Use PCI to make disks list
	disk_new_ata(ATA_PRIMARY_BUS, ATA_PRIMARY_CTRL);
}

static void disk_set_io_funcs(const disk_type_t type)
{
	switch(type)
	{
		case DISK_TYPE_ATA:
		{
			read_func = (disk_read_func_t) ata_read;
			write_func = (disk_write_func_t) ata_write;
			break;
		}

		// TODO

		default:
		{
			read_func = NULL;
			write_func = NULL;
			break;
		}
	}
}

// TODO If partition didn't change, do nothing
void disk_select_disk(const disk_t *disk)
{
	spin_lock(&spinlock);
	if(!disk || !disk->disk_struct)
	{
		current_device = NULL;
		goto end;
	}
	current_device = disk->disk_struct;
	disk_set_io_funcs(disk->type);
	start_lba = 0;
	if((end_lba = disk->sectors) == 0)
		current_device = NULL;

end:
	spin_unlock(&spinlock);
}

// TODO If partition didn't change, do nothing
void disk_select_partition(const partition_t *partition)
{
	spin_lock(&spinlock);
	if(!partition || !partition->disk || !partition->disk->disk_struct)
	{
		current_device = NULL;
		goto end;
	}
	current_device = partition->disk->disk_struct;
	disk_set_io_funcs(partition->disk->type);
	start_lba = partition->start_lba;
	end_lba = start_lba + partition->sectors;
	if(end_lba <= start_lba)
		current_device = NULL;

end:
	spin_unlock(&spinlock);
}

int disk_read(const size_t sector, char *buff, const size_t sectors_count)
{
	size_t i;

	if(!buff || sectors_count == 0 || sector >= end_lba - start_lba)
		return -1;
	if(!current_device || !read_func)
		return -1;
	spin_lock(&spinlock);
	for(i = 0; i < sectors_count; i += 0xff)
		read_func(current_device, start_lba + sector + i,
			buff + (i * ATA_SECTOR_SIZE), min(sectors_count - i, 0xff));
	spin_unlock(&spinlock);
	return (errno ? -1 : 0);
}

int disk_write(const size_t sector, const char *buff,
	const size_t sectors_count)
{
	size_t i;

	if(!buff || sectors_count == 0 || sector >= end_lba - start_lba)
		return -1;
	if(!current_device || !write_func)
		return -1;
	spin_lock(&spinlock);
	for(i = 0; i < sectors_count; i += 0xff)
		write_func(current_device, start_lba + sector + i,
			buff + (i * ATA_SECTOR_SIZE), min(sectors_count - i, 0xff));
	spin_unlock(&spinlock);
	return (errno ? -1 : 0);
}
