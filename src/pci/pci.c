#include <pci/pci.h>

uint16_t pci_config_readword(const uint8_t bus, const uint8_t device,
	const uint8_t func, const uint8_t offset)
{
	outl(PCI_CONFIG_ADDRESS, ((uint32_t) bus << 16) | ((uint32_t) device << 11)
		| ((uint32_t) func << 8) | ((uint32_t) offset & 0xfc) | 0x80000000);
	return (inl(PCI_CONFIG_DATA) >> ((offset & 2) * 8)) & 0xffff;
}

uint16_t pci_get_vendor_id(const uint8_t bus, const uint8_t device,
	const uint8_t func)
{
	return pci_config_readword(bus, device, func, 0x0);
}

uint16_t pci_get_device_id(const uint8_t bus, const uint8_t device)
{
	return pci_config_readword(bus, device, 0x0, 0x2);
}

uint16_t pci_get_header_type(const uint8_t bus, const uint8_t device)
{
	return pci_config_readword(bus, device, 0x3, 0xe);
}

static int pci_check_function(const uint8_t bus, const uint8_t device,
	const uint8_t func)
{
	uint16_t word;
	uint8_t class_code, subclass;

	if(pci_get_vendor_id(bus, device, func) == 0xffff)
		return 0;
	word = pci_config_readword(bus, device, func, 0xa);
	class_code = (word >> 8) & 0xff;
	subclass = word & 0xff;
	printf("class_code: %i; subclass: %i\n", class_code, subclass);
	// TODO
	return 1;
}

static int pci_check_device(const uint8_t bus, const uint8_t device)
{
	uint16_t vendor_id;
	uint8_t func = 0;

	if((vendor_id = pci_get_vendor_id(bus, device, func)) == 0xffff)
		return 0;
	if(!(pci_check_function(bus, device, func)))
		return 0;
	if(pci_get_header_type(bus, device) & 0x80)
	{
		for(func = 1; func < 8; ++func)
		{
			if(pci_check_function(bus, device, func))
				printf("CCCC\n");
		}
	}
	return 1;
}

static int pic_check_bus(const uint8_t bus)
{
	uint8_t device;

	if(pci_get_vendor_id(bus, 0, 0) == 0xffff)
		return 0;
	for(device = 0; device < 32; ++device)
	{
		if(pci_check_device(bus, device))
			printf("BBBB\n");
	}
	return 1;
}

// TODO Save peripherals
void pci_scan(void)
{
	uint16_t bus;

	// TODO
	printf("scan\n");
	for(bus = 0; bus < 256; ++bus)
	{
		if(pic_check_bus(bus))
			printf("AAAA\n");
	}
}
