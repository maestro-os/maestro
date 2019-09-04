#include <ata/ata.h>

static inline int ata_check_floating_bus(const uint16_t bus)
{
	return (inb(bus + ATA_STATUS_REG) == 0xff);
}

static inline int ata_is_busy(const uint16_t bus)
{
	return (inb(bus + ATA_STATUS_REG) & ATA_STATUS_BSY);
}

static inline int ata_identify(const uint16_t bus, const int slave,
	uint16_t *init_data)
{
	uint8_t status;
	size_t i;

	outb(bus + ATA_DRIVE_REG, (slave ? 0xa0 : 0xb0));
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

__attribute__((cold))
void ata_init(void)
{
	uint16_t init_data[256];
	uint32_t sectors = 0;

	if(ata_check_floating_bus(ATA_PRIMARY_BUS))
	{
		printf("ATA floating bus detected\n");
		return;
	}
	if(!ata_identify(ATA_PRIMARY_BUS, 0, init_data))
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
	// TODO
	printf("ATA initialized!\n");
}

void ata_reset(const uint16_t ctrl_bus)
{
	uint8_t reg;

	reg = ctrl_bus + ATA_CTRL_DEVICE_CONTROL_REG;
	outb(reg, inb(reg) | 0b100);
	outb(reg, inb(reg) & ~0b100);
}
