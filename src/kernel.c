#include <kernel.h>
#include <tty/tty.h>
#include <cpu/cpu.h>
#include <memory/memory.h>
#include <idt/idt.h>
#include <pit/pit.h>
#include <acpi/acpi.h>
#include <pci/pci.h>
#include <process/process.h>
#include <device/device.h>
#include <ata/ata.h>

#include <libc/stdio.h>

// TODO temporary
#include <pic/pic.h>
#include <libc/errno.h>

static driver_t drivers[] = {
	{"PS/2", ps2_init},
	{"ATA", ata_init}
};

#ifdef KERNEL_DEBUG
static void print_devices(void)
{
	pci_function_t *f;

	f = pci_get_all();
	printf("PCI devices:\n");
	while(f)
	{
		printf("- vendor_id: %x; device_id: %x; class_code: %x; subclass: %x; \
prog_if: %x; revision_id: %x; bar0: %x; bar1: %x\n",
			f->vendor_id, f->device_id, f->class_code, f->subclass,
				f->prog_if, f->revision_id, (int) f->bar0, (int) f->bar1);
		f = f->next;
	}
	printf("\n");
}

static void print_slabs(void)
{
	cache_t *c;

	printf("--- Slab allocator caches ---\n");
	printf("<name> <slabs> <objsize> <objects_count>\n");
	c = cache_getall();
	while(c)
	{
		printf("%s %u %u %u\n", c->name, (unsigned) c->slabs,
			(unsigned) c->objsize, (unsigned) c->objcount); // TODO Use %zu
		c = c->next;
	}
	printf("\n");
}
#endif

__attribute__((cold))
static inline void init_driver(const driver_t *driver)
{
	if(!driver)
		return;
	printf("%s driver initialization...\n", driver->name);
	driver->init_func();
}

__attribute__((cold))
static inline void init_drivers(void)
{
	size_t i = 0;

	// TODO Load drivers according to detected devices
	while(i < sizeof(drivers) / sizeof(*drivers))
		init_driver(drivers + i++);
}

// TODO Remove
void test_process(void);

__attribute__((cold))
void kernel_main(const unsigned long magic, void *multiboot_ptr,
	void *kernel_end)
{
	boot_info_t boot_info;

	// TODO Fix
	if(!check_a20())
		enable_a20();
	tty_init();
	idt_init();
	pit_init();

	if(magic != MULTIBOOT2_BOOTLOADER_MAGIC)
		PANIC("Non Multiboot2-compliant bootloader!", 0);
	if(((uintptr_t) multiboot_ptr) & 7)
		PANIC("Boot informations structure's address is not aligned!", 0);

	printf("Booting crumbleos kernel version %s...\n", KERNEL_VERSION);
	printf("Retrieving CPU informations...\n");
	cpuid();

	printf("Retrieving Multiboot2 data...\n");
	read_boot_tags(multiboot_ptr, &boot_info);
	printf("Command line: %s\n", boot_info.cmdline);
	printf("Bootloader name: %s\n", boot_info.loader_name);

	printf("Memory management initialization...\n");
	memmap_init(&boot_info, multiboot_ptr, kernel_end);
#ifdef KERNEL_DEBUG
	memmap_print();
	printf("\n");
#endif
	printf("Available memory: %u bytes (%u pages)\n",
		(unsigned) available_memory, (unsigned) available_memory / PAGE_SIZE);
	printf("Kernel end: %p; Heap end: %p\n", kernel_end, heap_end);
	buddy_init();
	printf("Buddy allocator begin: %p\n", buddy_begin);
	slab_init();
	vmem_kernel(&boot_info);

	printf("ACPI initialization...\n");
	acpi_init();

	// TODO PCIe
	printf("PCI initialization...\n");
	pci_scan();
#ifdef KERNEL_DEBUG
	print_devices();
#endif

	printf("Drivers initialization...\n");
	init_drivers();

	printf("Keyboard initialization...\n");
	keyboard_init();
	keyboard_set_input_hook(tty_input_hook);
	keyboard_set_ctrl_hook(tty_ctrl_hook);
	keyboard_set_erase_hook(tty_erase_hook);

	printf("Processes initialization...\n");
	process_init();

#ifdef KERNEL_DEBUG
	print_mem_usage();
	print_slabs();
#endif

	// TODO Test
	CLI();
	for(size_t i = 0; i < 1; ++i)
		new_process(NULL, test_process);

	kernel_loop();
}
