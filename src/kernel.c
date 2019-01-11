#include "kernel.h"
#include "multiboot.h"
#include "tty/tty.h"

void kernel_main(const void* bi, const gdt_descriptor_t* gdt)
{
	(void) bi;
	// boot_info = load_boot_info(bi);

	(void) gdt;

	tty_init();

	// TODO test
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
