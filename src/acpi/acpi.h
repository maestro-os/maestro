#ifndef ACPI_H
# define ACPI_H

# define RSDP_LOOKUP_BEGIN	((void *) 0xe0000)
# define RSDP_LOOKUP_END	((void *) 0xfffff)
# define RSDP_SIGNATURE		"RSD PTR "

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

typedef struct rsdp_desc rsdp_desc_t;
typedef struct rsdp20_desc rsdp20_desc_t;

rsdp_desc_t *rsdp_find(void);

void acpi_init(void);

#endif
