#include <pci/pci.h>

static cache_t *pci_cache = NULL;
static pci_function_t *functions = NULL;

uint16_t pci_config_readlong(const uint8_t bus, const uint8_t device,
	const uint8_t func, const uint8_t offset)
{
	outl(PCI_CONFIG_ADDRESS, ((uint32_t) bus << 16) | ((uint32_t) device << 11)
		| ((uint32_t) func << 8) | ((uint32_t) offset & 0xfc) | 0x80000000);
	return inl(PCI_CONFIG_DATA);
}

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
	uint16_t vendor_id;
	pci_function_t *function;
	uint32_t i;

	if((vendor_id = pci_get_vendor_id(bus, device, func)) == 0xffff)
		return 0;
	if(!(function = cache_alloc(pci_cache)))
	{
		// TODO Error
		return 0;
	}
	function->bus = bus;
	function->device = device;
	function->function = func;
	function->vendor_id = vendor_id;
	function->device_id = pci_get_device_id(bus, device);
	i = pci_config_readword(bus, device, func, 0xa);
	function->class_code = (i >> 8) & 0xff;
	function->subclass = i & 0xff;
	i = pci_config_readword(bus, device, func, 0x8);
	function->prog_if = (i >> 8) & 0xff;
	function->revision_id = i & 0xff;
	function->bar0 = pci_config_readlong(bus, device, func, 0x10);
	function->bar1 = pci_config_readlong(bus, device, func, 0x14);
	// TODO interrupt_line and interrupt_pin
	function->next = functions;
	functions = function;
	return 1;
}

static void pci_check_device(const uint8_t bus, const uint8_t device)
{
	uint16_t vendor_id;
	uint8_t func = 0;

	if((vendor_id = pci_get_vendor_id(bus, device, func)) == 0xffff)
		return;
	if(!(pci_check_function(bus, device, func)))
		return;
	if(pci_get_header_type(bus, device) & 0x80)
	{
		for(func = 1; func < 8; ++func)
			pci_check_function(bus, device, func);
	}
	return;
}

static void pic_check_bus(const uint8_t bus)
{
	uint8_t device;

	if(pci_get_vendor_id(bus, 0, 0) == 0xffff)
		return;
	for(device = 0; device < 32; ++device)
		pci_check_device(bus, device);
	return;
}

void pci_scan(void)
{
	uint16_t bus;

	if(!pci_cache && !(pci_cache = cache_create("pci", sizeof(pci_function_t),
		1, bzero, NULL)))
		PANIC("Cannot allocate caches for PCI!", 0);
	for(bus = 0; bus < 256; ++bus)
		pic_check_bus(bus);
}

pci_function_t *pci_get_all(void)
{
	return functions;
}
