#include <ata/ata.h>

__attribute__((cold))
void ata_init(void)
{
	ata_device_t dev;

	bzero(&dev, sizeof(ata_device_t));
	dev.bus = ATA_PRIMARY_BUS;
	dev.ctrl = ATA_PRIMARY_CTRL;
	ata_init_device(&dev);
	// TODO Printfs
}

static inline void ata_wait(const uint16_t port)
{
	size_t i;

	for(i = 0; i < 4; ++i)
		inb(port);
}

static inline int ata_check_floating_bus(const uint16_t bus)
{
	return (inb(bus + ATA_STATUS_REG) == 0xff);
}

static inline int ata_is_busy(const uint16_t bus)
{
	return (inb(bus + ATA_STATUS_REG) & ATA_STATUS_BSY);
}

static inline void ata_select_drive(const uint16_t bus, const int slave)
{
	outb(bus + ATA_DRIVE_REG, slave ? 0xa0 : 0xb0);
}

static inline int ata_identify(const uint16_t bus, const int slave,
	uint16_t *init_data)
{
	uint8_t status;
	size_t i;

	ata_select_drive(bus, slave);
	outb(bus + ATA_SECTOR_COUNT_REG, 0x0);
	outb(bus + ATA_SECTOR_NUMBER_REG, 0x0);
	outb(bus + ATA_CYLINDER_LOW_REG, 0x0);
	outb(bus + ATA_CYLINDER_HIGH_REG, 0x0);
	outb(bus + ATA_COMMAND_REG, ATA_CMD_IDENTIFY);
	if((status = inb(bus + ATA_STATUS_REG)) == 0)
		return 0;
	while(ata_is_busy(bus))
		;
	if(inb(bus + ATA_CYLINDER_LOW_REG) || inb(bus + ATA_CYLINDER_HIGH_REG))
		return 0;
	do
	{
		status = inb(bus + ATA_STATUS_REG);
	}
	while(!(status & ATA_STATUS_ERR) && !(status & ATA_STATUS_DRQ));
	// TODO Some ATAPI devices doesn't set ERR on abort
	if(status & ATA_STATUS_ERR)
		return 0;
	bzero(init_data, 256 * sizeof(uint16_t));
	for(i = 0; i < 256; ++i)
		init_data[i] = inw(bus + ATA_DATA_REG);
	return 1;
}

static inline uint32_t ata_lba28_sectors(const uint16_t *data)
{
	return *(uint32_t *) (data + 60);
}

static inline int ata_supports_lba48(const uint16_t *data)
{
	return (data[83] & 0b10000000000);
}

// TODO Put printfs out of this functions
void ata_init_device(ata_device_t *dev)
{
	uint16_t init_data[256];
	uint32_t sectors = 0;

	if(!dev)
		return;
	if(ata_check_floating_bus(dev->bus))
	{
		printf("ATA floating bus detected\n");
		return;
	}
	if(!ata_identify(dev->bus, 0, init_data))
	{
		printf("ATA identify failed\n");
		return;
	}
	if((sectors = ata_lba28_sectors(init_data)) != 0)
		printf("ATA LBA28 sectors: %u\n", (unsigned) sectors);
	if(ata_supports_lba48(init_data))
	{
		printf("ATA LBA48 supported\n");
		// TODO Get sectors
	}
	printf("ATA disk size: %u bytes\n", (unsigned) sectors * ATA_SECTOR_SIZE);
	// TODO Set data in struct
	printf("ATA initialized!\n");
}

int ata_get_type(const ata_device_t *dev, const int slave)
{
	unsigned cl, ch;

	if(!dev)
		return ATA_TYPE_UNKNOWN;
	ata_reset(dev);
	ata_select_drive(dev->bus, slave);
	ata_wait(dev->ctrl);
	cl = inb(dev->bus + ATA_CYLINDER_LOW_REG);
	ch = inb(dev->bus + ATA_CYLINDER_HIGH_REG);
	if(cl == 0 && ch == 0)
		return ATA_TYPE_PATA;
	if(cl == 0x14 && ch == 0xeb)
		return ATA_TYPE_PATAPI;
	if(cl == 0x3c && ch == 0xc3)
		return ATA_TYPE_SATA;
	if(cl == 0x69 && ch == 0x96)
		return ATA_TYPE_SATAPI;
	return ATA_TYPE_UNKNOWN;
}

// TODO Set errnos?
int ata_read(const ata_device_t *dev, const int slave, const size_t lba,
	void *buff, const size_t sectors)
{
	size_t i;

	if(!dev || !buff || sectors > 0xff)
		return -1;
	outb(dev->bus + ATA_DRIVE_REG, (slave ? 0xe0 : 0xf0)
		| ((lba >> 24) & 0xf));
	outb(dev->bus + ATA_SECTOR_COUNT_REG, (uint8_t) sectors);
	outb(dev->bus + ATA_SECTOR_NUMBER_REG, (uint8_t) lba);
	outb(dev->bus + ATA_CYLINDER_LOW_REG, (uint8_t) (lba >> 8));
	outb(dev->bus + ATA_CYLINDER_HIGH_REG, (uint8_t) (lba >> 16));
	outb(dev->bus + ATA_COMMAND_REG, ATA_CMD_READ_SECTORS);
	for(i = 0; i < sectors; ++i)
	{
		// TODO Wait for IRQ (or poll?)
		// TODO `return -1;` on error
		// TODO Read
		if(i >= sectors)
			ata_wait(dev->ctrl);
	}
	return 0;
}

// TODO ata_write

void ata_reset(const ata_device_t *dev)
{
	uint8_t reg;

	if(!dev)
		return;
	reg = dev->ctrl + ATA_CTRL_DEVICE_CONTROL_REG;
	outb(reg, inb(reg) | 0b100);
	outb(reg, inb(reg) & ~0b100);
}
