#ifndef PCI_H
# define PCI_H

# include <kernel.h>

# define PCI_CONFIG_ADDRESS	0xcf8
# define PCI_CONFIG_DATA	0xcfc

// TODO Move somewhere else?
# define PIN_INT_NONE	0x0
# define PIN_INTA		0x1
# define PIN_INTB		0x2
# define PIN_INTC		0x3
# define PIN_INTD		0x4

typedef struct pci_function
{
	struct pci_function *next;

	uint8_t bus;
	uint8_t device;
	uint8_t function;

	uint16_t vendor_id;
	uint16_t device_id;

	uint8_t class_code;
	uint8_t subclass;
	uint8_t prog_if;
	uint8_t revision_id;

	uint32_t bar0;
	uint32_t bar1;

	uint8_t interrupt_line;
	uint8_t interrupt_pin;
} pci_function_t;

uint16_t pci_config_readlong(uint8_t bus, uint8_t device,
	uint8_t func, uint8_t offset);
uint16_t pci_config_readword(uint8_t bus, uint8_t device,
	uint8_t func, uint8_t offset);

uint16_t pic_get_vendor_id(uint8_t bus, uint8_t device, uint8_t func);
uint16_t pic_get_device_id(uint8_t bus, uint8_t device);
uint16_t pic_get_header_type(uint8_t bus, uint8_t device);

void pci_scan(void);

pci_function_t *pci_get_all(void);

#endif
