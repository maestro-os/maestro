#include <kernel.h>
#include <tty/tty.h>
#include <cpu/cpu.h>
#include <memory/memory.h>
#include <memory/vmem/vmem.h>
#include <idt/idt.h>
#include <pit/pit.h>
#include <acpi/acpi.h>
#include <pci/pci.h>
#include <cmos/cmos.h>
#include <disk/ata/ata.h>
#include <disk/disk.h>
#include <keyboard/keyboard.h>
#include <process/process.h>

#include <libc/stdio.h>

// TODO temporary
#include <disk/ext2/ext2.h>
#include <libc/errno.h>

#ifdef KERNEL_SELFTEST
# include <selftest/selftest.h>
#endif

/*
 * The list of default drivers to be loaded by the kernel.
 */
static driver_t drivers[] = {
	{"PS/2", ps2_init},
	{"ATA", ata_init}
};

#ifdef KERNEL_DEBUG
/*
 * Prints all PCI devices found.
 */
// TODO Uncomment
/*static void print_devices(void)
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
}*/

/*
 * Prints all slabs of the slab allocator.
 */
static void print_slabs(void)
{
	cache_t *c;

	printf("--- Slab allocator caches ---\n");
	printf("<name> <objsize> <objects_count>\n");
	c = cache_getall();
	while(c)
	{
		printf("%s %zu %zu\n", c->name, c->objsize, c->objcount);
		c = c->next;
	}
	printf("\n");
}
#endif

/*
 * Initializes the given driver.
 */
ATTR_COLD
static inline void init_driver(const driver_t *driver)
{
	if(!driver)
		return;
	printf("%s driver initialization...\n", driver->name);
	driver->init_func();
}

/*
 * Initializes drivers.
 */
ATTR_COLD
static inline void init_drivers(void)
{
	size_t i = 0;

	// TODO Load drivers according to detected devices
	while(i < sizeof(drivers) / sizeof(*drivers))
		init_driver(drivers + i++);
}

// TODO Remove
void test_process(void);

// TODO Remove
extern semaphore_t sem;

/*
 * The kernel's main function. `magic` is the magic number given by Multiboot to
 * be checked by this function. `multiboot_ptr` is the pointer to the structure
 * containing boot informations. `kernel_end` is the pointer to the end of the
 * kernel image.
 */
ATTR_COLD
void kernel_main(const unsigned long magic, void *multiboot_ptr)
{
	tty_init();

	if(magic != MULTIBOOT2_BOOTLOADER_MAGIC || !IS_ALIGNED(multiboot_ptr, 8))
		PANIC("Non Multiboot2-compliant bootloader!", 0);

	idt_init();
	pit_init();

	printf("Booting Maestro kernel version %s...\n", KERNEL_VERSION);
	cpuid();
	read_boot_tags(multiboot_ptr);
	printf("Command line: %s\n", boot_info.cmdline);
	printf("Bootloader name: %s\n", boot_info.loader_name);

	memmap_init(multiboot_ptr);
#ifdef KERNEL_DEBUG
	memmap_print();
	printf("\n");
#endif
	printf("Available memory: %zu bytes (%zu pages)\n",
		mem_info.available_memory, mem_info.available_memory / PAGE_SIZE);
	buddy_init();
	slab_init();
	vmem_kernel();

	// TODO Move back after driver init
	printf("Keyboard initialization...\n");
	ps2_init(); // TODO rm
	keyboard_init();
	keyboard_set_input_hook(tty_input_hook);
	keyboard_set_ctrl_hook(tty_ctrl_hook);
	keyboard_set_erase_hook(tty_erase_hook);

#ifdef KERNEL_SELFTEST
	printf("Running selftests...\n");
	run_selftest();
	kernel_loop();
#endif

	printf("ACPI initialization...\n");
	// TODO acpi_init();

	// TODO PCIe
	printf("PCI initialization...\n");
	pci_scan();

	printf("Clock initialization...\n");
	time_init();

	printf("Drivers initialization...\n");
	init_drivers();

	printf("Disks initialization...\n");
	disk_init();

	printf("Processes initialization...\n");
	process_init();

	// TODO Remove
	CLI();
	sem_init(&sem);
	for(size_t i = 0; i < 1; ++i)
	{
		regs_t r;
		bzero(&r, sizeof(r));
		r.eip = (intptr_t)test_process;
		if(!process_create(NULL, &r))
			printf("process creation failed!\n");
	}

#ifdef KERNEL_DEBUG
	print_slabs();
	print_mem_usage();
#endif

	// TODO Remove
	/*partition_create(disks, EXT2_PARTITION_TYPE);

	char buff[ATA_SECTOR_SIZE];
	bzero(buff, sizeof(buff));
	disk_select_disk(disks);
	if(disk_read(0, buff, 1) < 0)
		printf("disk read err\n");
	tty_write(buff, sizeof(buff), current_tty);*/

	kernel_loop();
}
