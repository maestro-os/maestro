#ifndef ATA_H
# define ATA_H

# include <kernel.h>

# include <libc/sys/types.h>

# define ATA_PRIMARY_BUS	0x1f0
# define ATA_PRIMARY_CTRL	0x3f6
# define ATA_SECONDARY_BUS	0x170
# define ATA_SECONDARY_CTRL	0x376

# define ATA_DATA_REG			0x0
# define ATA_ERROR_REG			0x1
# define ATA_FEATURES_REG		0x1
# define ATA_SECTOR_COUNT_REG	0x2
# define ATA_SECTOR_NUMBER_REG	0x3
# define ATA_CYLINDER_LOW_REG	0x4
# define ATA_CYLINDER_HIGH_REG	0x5
# define ATA_DRIVE_REG			0x6
# define ATA_STATUS_REG			0x7
# define ATA_COMMAND_REG		0x7

# define ATA_CTRL_ALTERNATE_STATUS_REG	0x0
# define ATA_CTRL_DEVICE_CONTROL_REG	0x0
# define ATA_CTRL_DRIVE_ADDRESS_REG		0x1

# define ATA_ERR_AMNF	0b00000001
# define ATA_ERR_TKZNF	0b00000010
# define ATA_ERR_ABRT	0b00000100
# define ATA_ERR_MCR	0b00001000
# define ATA_ERR_IDNF	0b00010000
# define ATA_ERR_MC		0b00100000
# define ATA_ERR_UNC	0b01000000
# define ATA_ERR_BBK	0b10000000

# define ATA_STATUS_ERR		0b00000001
# define ATA_STATUS_IDX		0b00000010
# define ATA_STATUS_CORR	0b00000100
# define ATA_STATUS_DRQ		0b00001000
# define ATA_STATUS_SRV		0b00010000
# define ATA_STATUS_DF		0b00100000
# define ATA_STATUS_RDY		0b01000000
# define ATA_STATUS_BSY		0b10000000

# define ATA_CMD_IDENTIFY		0xec
# define ATA_CMD_READ_SECTORS	0x20
# define ATA_CMD_WRITE_SECTORS	0x30

// TODO Might be different from disk to disk
# define ATA_SECTOR_SIZE	0x200

# define ATA_TYPE_UNKNOWN	0x0
# define ATA_TYPE_PATA		0x1
# define ATA_TYPE_PATAPI	0x2
# define ATA_TYPE_SATA		0x3
# define ATA_TYPE_SATAPI	0x4

typedef struct ata_device
{
	struct ata_device *next;

	uint16_t bus;
	uint16_t ctrl;

	// TODO

	spinlock_t spinlock;
	int wait_irq;
} ata_device_t;

extern ata_device_t *devices;

void ata_init(void);
void ata_irq(void);
void ata_err_check(void);

ata_device_t *ata_init_device(const uint16_t bus, const uint16_t ctrl);
int ata_get_type(const ata_device_t *dev, int slave);
int ata_read(ata_device_t *dev, int slave, size_t lba,
	void *buff, size_t sectors);
// TODO ata_write
void ata_reset(const ata_device_t *dev);

#endif
