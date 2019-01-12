#include "kernel.h"
#include "tty/tty.h"

#include "libc/stdio.h"

void kernel_main(const unsigned long magic, const void* ptr)
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

	printf("Booting kernel...\n");
	printf("Retrieving Multiboot data...\n");

	read_boot_tags(ptr);

	// TODO
}

__attribute((noreturn))
void panic(const char* reason)
{
	tty_init();
	printf("--- KERNEL PANIC ---\n\nKernel has been forced to halt due to internal problem, sorry :/\nReason: %s\n\nIf you belive this is a bug on the kernel side, please feel free to report it.", reason);

	kernel_halt();
}
