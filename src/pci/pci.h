#ifndef PCI_H
# define PCI_H

# include <kernel.h>

# define PCI_CONFIG_ADDRESS	0xcf8
# define PCI_CONFIG_DATA	0xcfc

uint16_t pci_config_readword(uint8_t bus, uint8_t device,
	uint8_t func, uint8_t offset);

uint16_t pic_get_vendor_id(uint8_t bus, uint8_t device, uint8_t func);
uint16_t pic_get_device_id(uint8_t bus, uint8_t device);
uint16_t pic_get_header_type(uint8_t bus, uint8_t device);

void pci_scan(void);

#endif
