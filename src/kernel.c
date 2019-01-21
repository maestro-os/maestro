#include "kernel.h"
#include "tty/tty.h"

#include "libc/stdio.h"

void kernel_main(const unsigned long magic, const void *ptr)
{
	tty_init();

	if(magic != MULTIBOOT2_BOOTLOADER_MAGIC) {
		panic("Non Multiboot2-compliant bootloader!");
		return;
	}

	if(((uintptr_t) ptr) & 7) {
		panic("Boot informations structure's address is not aligned!");
		return;
	}

	printf("Booting crumbleos kernel version %s...\n", KERNEL_VERSION);
	printf("Retrieving Multiboot data...\n");

	const boot_info_t boot_info = read_boot_tags(ptr);

	printf("Command line: %s\n", boot_info.cmdline);
	printf("Bootloader name: %s\n", boot_info.loader_name);
	printf("Memory lower bound: %u\n", boot_info.mem_lower);
	printf("Memory upper bound: %u\n", boot_info.mem_upper);
}

__attribute((noreturn))
void panic(const char *reason)
{
	tty_init();
	printf("--- KERNEL PANIC ---\n\nKernel has been forced to halt due to internal problem, sorry :/\nReason: %s\n\nIf you belive this is a bug on the kernel side, please feel free to report it.", reason);

	kernel_halt();
}
