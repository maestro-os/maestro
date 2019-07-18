#include "kernel.h"
#include "tty/tty.h"
#include "cpu/cpu.h"
#include "memory/memory.h"
#include "memory/kmalloc_internal.h"
#include "idt/idt.h"
#include "pit/pit.h"
#include "process/process.h"
#include "device/device.h"
#include "ata/ata.h"

#include "libc/stdio.h"

// TODO temporary
#include "libc/errno.h"

static const char *errors[] = {
	"Divide-by-zero Error",
	"Debug",
	"Non-maskable Interrupt",
	"Breakpoint",
	"Overflow",
	"Bound Range Exceeded",
	"Invalid Opcode",
	"Device Not Available",
	"Double Fault",
	"Coprocessor Segment Overrun",
	"Invalid TSS",
	"Segment Not Present",
	"Stack-Segment Fault",
	"General Protection Fault",
	"Page Fault",
	"Unknown",
	"x87 Floating-Point Exception",
	"Alignement Check",
	"Machine Check",
	"SIMD Floating-Point Exception",
	"Virtualization Exception",
	"Unknown",
	"Unknown",
	"Unknown",
	"Unknown",
	"Unknown",
	"Unknown",
	"Unknown",
	"Unknown",
	"Unknown",
	"Security Exception",
	"Unknown"
};

static driver_t drivers[] = {
	{"PS/2", ps2_init},
	{"ATA", ata_init}
};

__attribute__((cold))
static inline void init_driver(const driver_t *driver)
{
	if(!driver) return;

	printf("%s driver initialization...\n", driver->name);
	driver->init_func();
}

__attribute__((cold))
static inline void init_drivers(void)
{
	for(size_t i = 0; i < sizeof(drivers) / sizeof(*drivers); ++i)
		init_driver(drivers + i);
}

// TODO Remove
static void test_process()
{
	printf("Test process runned!\n");
	while(1)
		;
	// TODO Let's see what happens when returning this function
}

__attribute__((cold))
void kernel_main(const unsigned long magic, void *multiboot_ptr,
	void *kernel_end)
{
	// TODO Fix
	if(!check_a20())
		enable_a20();

	tty_init();

	// TODO Add first Multiboot version support
	if(magic != MULTIBOOT2_BOOTLOADER_MAGIC)
		PANIC("Non Multiboot2-compliant bootloader!", 0);

	if(((uintptr_t) multiboot_ptr) & 7)
		PANIC("Boot informations structure's address is not aligned!", 0);

	printf("Booting crumbleos kernel version %s...\n", KERNEL_VERSION);
	printf("Retrieving CPU informations...\n");

	// TODO
	//cpuid();

	printf("Retrieving Multiboot2 data...\n");

	const boot_info_t boot_info = read_boot_tags(multiboot_ptr);

	printf("Command line: %s\n", boot_info.cmdline);
	printf("Bootloader name: %s\n", boot_info.loader_name);
	printf("Basic components initialization...\n");

	idt_init();
	pit_init();

	printf("Memory management initialization...\n");

#ifdef KERNEL_DEBUG
	printf("--- Memory mapping ---\n");
	printf("<begin> <end> <type>\n");

	for(size_t i = 0; i < memory_maps_count; ++i)
	{
		const multiboot_mmap_entry_t *t = memory_maps + i;
		printf("- %p %p %s\n", (void *) (uintptr_t) t->addr,
			(void *) (uintptr_t) t->addr + t->len, memmap_type(t->type));
	}

	printf("\n");
#endif

	heap_begin = kernel_end;
	heap_end = (void *) (boot_info.mem_upper * 1024);
	available_memory = heap_end - heap_begin;

	printf("Available memory: %u bytes (%u pages)\n",
		(unsigned) available_memory, (unsigned) available_memory / PAGE_SIZE);
	printf("Kernel end: %p; Heap end: %p\n", kernel_end, heap_end);

	buddy_init();

	printf("Buddy allocator begin: %p\n", buddy_begin);

	slab_init();
	kmalloc_init();

#ifdef KERNEL_DEBUG
	printf("--- Slab allocator caches ---\n");
	printf("<name> <slabs> <objsize> <objects_count>\n");

	cache_t *c = cache_getall();

	while(c)
	{
		printf("%s %u %u %u\n", c->name, (unsigned )c->slabs,
			(unsigned) c->objsize, (unsigned) c->objects_count); // TODO Use %zu
		c = c->next;
	}

	printf("\n");
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

	// TODO Test
	errno = 0;
	process_t *proc = new_process(NULL, test_process);
	printf("pid: %i, errno: %i\n", (int) proc->pid, (int) errno);

	idt_set_state(true);
	kernel_loop();
}

void error_handler(const unsigned error, const uint32_t error_code)
{
	if(error > 0x1f)
		PANIC("Unknown", error_code);

	// TODO Check if caused by process
	PANIC(errors[error], error_code);
}

__attribute__((cold))
static void print_panic(const char *reason, const uint32_t code)
{
	tty_init();
	printf("--- KERNEL PANIC ---\n\n");
	printf("Kernel has been forced to halt due to internal problem,\
 sorry :/\n");
	printf("Reason: %s\n", reason);
	printf("Error code: %x\n\n", (unsigned) code);
	printf("If you believe this is a bug on the kernel side,\
 please feel free to report it.\n");
}

__attribute__((cold))
__attribute((noreturn))
void kernel_panic(const char *reason, const uint32_t code)
{
	print_panic(reason, code);
	kernel_halt();
}

__attribute__((cold))
__attribute__((noreturn))
void kernel_panic_(const char *reason, const uint32_t code,
	const char *file, const int line)
{
	print_panic(reason, code);
	printf("\n-- DEBUG --\nFile: %s; Line: %i", file, line);
	kernel_halt();
}
