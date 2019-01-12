#include "kernel.h"
#include "multiboot.h"
#include "tty/tty.h"

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

	read_boot_tags(ptr);

	// test
	tty_write("Hello world!", 12);
}

__attribute((noreturn))
void panic(const char* reason)
{
	const char* message = "--- KERNEL PANIC ---\n\nKernel has been forced to halt due to internal problem, sorry :/\nReason: ";
	const char* second_message = "\n\nIf you belive this is a bug on the kernel side, please feel free to report it.";

	tty_init();
	tty_write(message, strlen(message));
	tty_write(reason, strlen(reason));
	tty_write(second_message, strlen(second_message));

	kernel_halt();
}
