#ifndef ACPI_H
# define ACPI_H

# define RSDP_LOOKUP_BEGIN	((void *) 0xe0000)
# define RSDP_LOOKUP_END	((void *) 0xfffff)
# define RSDP_SIGNATURE		"RSD PTR "

# define ACPI_ENTRIES_COUNT(table)\
	(((table)->header.length - sizeof(acpi_sdt_header_t))\
		/ sizeof(*((table)->entries)))

# include <kernel.h>

__attribute__ ((packed))
struct rsdp_desc
{
	char signature[8];
	uint8_t checksum;
	char OEMID[6];
	uint8_t revision;
	uint32_t rsdt_address;
};

__attribute__((packed))
struct rsdp20_desc
{
	struct rsdp_desc desc;
	uint32_t length;
	uint64_t xsdt_address;
	uint8_t extended_checksum;
	uint8_t reserved[3];
};

__attribute__((packed))
struct acpi_sdt_header
{
	char signature[4];
	uint32_t length;
	uint8_t revision;
	uint8_t checksum;
	char OEM_id[6];
	char OEM_tableid[8];
	uint32_t OEM_revision;
	uint32_t creator_id;
	uint32_t creator_revision;
};

struct generic_address_structure
{
  uint8_t address_space;
  uint8_t bit_width;
  uint8_t bit_offset;
  uint8_t access_size;
  uint64_t address;
};

__attribute__((packed))
struct rsdt
{
	struct acpi_sdt_header header;
	uint32_t entries[0];
};

__attribute__((packed))
struct xsdt
{
	struct acpi_sdt_header header;
	uint64_t entries[0];
};

__attribute__((packed))
struct madt
{
	struct acpi_sdt_header header;
	// TODO
};

struct fadt
{
	struct acpi_sdt_header header;
	uint32_t firmware_ctrl;
	uint32_t dsdt;

	uint8_t reserved;

	uint8_t preferred_power_management_profile;
	uint16_t sci_interrupt;
	uint32_t smi_command_port;
	uint8_t acpi_enable;
	uint8_t acpi_disable;
	uint8_t s4bios_req;
	uint8_t pstate_control;
	uint32_t pm1a_event_block;
	uint32_t pm1b_event_block;
	uint32_t pm1a_control_block;
	uint32_t pm1b_control_block;
	uint32_t pm2_control_block;
	uint32_t pm_timer_block;
	uint32_t gpe0_block;
	uint32_t gpe1_block;
	uint8_t pm1_event_length;
	uint8_t pm1_control_length;
	uint8_t pm2_control_length;
	uint8_t pm_timer_length;
	uint8_t gpe0_length;
	uint8_t gpe1_length;
	uint8_t gpe1_base;
	uint8_t cstate_control;
	uint16_t worstc2_latency;
	uint16_t worstc3_latency;
	uint16_t flush_size;
	uint16_t flush_stride;
	uint8_t duty_offset;
	uint8_t duty_width;
	uint8_t day_alarm;
	uint8_t month_alarm;
	uint8_t century;

	uint16_t bootarchitectureflags;

	uint8_t reserved2;
	uint32_t flags;

	struct generic_address_structure resetreg;

	uint8_t resetvalue;
	uint8_t reserved3[3];

	uint64_t X_firmware_control;
	uint64_t X_dsdt;

	struct generic_address_structure X_PM1a_eventblock;
	struct generic_address_structure X_PM1b_eventblock;
	struct generic_address_structure X_PM1a_controlblock;
	struct generic_address_structure X_PM1b_controlblock;
	struct generic_address_structure X_PM2_controlblock;
	struct generic_address_structure X_PM_timerblock;
	struct generic_address_structure X_GPE0_block;
	struct generic_address_structure X_GPE1_block;
};

typedef struct rsdp_desc rsdp_desc_t;
typedef struct rsdp20_desc rsdp20_desc_t;
typedef struct acpi_sdt_header acpi_sdt_header_t;
typedef struct generic_address_structure generic_address_structure_t;
typedef struct rsdt rsdt_t;
typedef struct xsdt xsdt_t;
typedef struct madt madt_t;
typedef struct fadt fadt_t;

typedef struct table_handle
{
	char signature[4];
	void (*handle)(void *);
} table_handle_t;

int signature_check(const char *desc_sign, const char *sign);
int checksum_check(void *desc, const size_t size);

rsdp_desc_t *rsdp_find(void);
void handle_rsdt(rsdt_t *rsdt);
void handle_xsdt(xsdt_t *xsdt);

void handle_madt(madt_t *madt);
void handle_fadt(fadt_t *fadt);

void acpi_init(void);

#endif
