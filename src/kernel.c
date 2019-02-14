#include "kernel.h"
#include "syscall/syscall.h"
#include "tty/tty.h"

#include "libc/stdio.h"

void kernel_main(const void *kernel, const unsigned long magic, const void *ptr)
{
	tty_init();

	if(magic != MULTIBOOT2_BOOTLOADER_MAGIC)
	{
		panic("Non Multiboot2-compliant bootloader!");
	}

	if(((uintptr_t) ptr) & 7)
	{
		panic("Boot informations structure's address is not aligned!");
	}

	printf("Booting crumbleos kernel version %s...\n", KERNEL_VERSION);
	printf("Kernel is loaded at address %p\n", kernel);
	printf("Retrieving Multiboot data...\n");

	const boot_info_t boot_info = read_boot_tags(ptr);

	printf("Command line: %s\n", boot_info.cmdline);
	printf("Bootloader name: %s\n", boot_info.loader_name);
	printf("Memory lower bound: %u\n", boot_info.mem_lower);
	printf("Memory upper bound: %u\n", boot_info.mem_upper);

	if((boot_info.mem_upper * 1024) <= 0x200000)
	{
		panic("No heap space for kernel!");
	}

	kernel_heap_end = (void *) (boot_info.mem_upper * 1024) - 0x200000;

	printf("\nKernel heap space has a size of: %i byte(s)\n",
		kernel_heap_end - KERNEL_HEAP_BEGIN);

	// TODO
}

__attribute((noreturn))
void panic(const char *reason)
{
	tty_init();
	printf("--- KERNEL PANIC ---\n\nKernel has been forced to halt due to internal problem, sorry :/\nReason: %s\n\nIf you belive this is a bug on the kernel side, please feel free to report it.", reason);

	kernel_halt();
}
