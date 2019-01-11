#include "kernel.h"
#include "tty/tty.h"
#include "vga/vga.h"

void kernel_main(const void* bi)
{
	(void) bi;
	// boot_info = load_boot_info(bi);

	tty_init();

	// TODO test
	const char* str = "\tHllo world!";
	for(size_t i = 0; i < 100; ++i)
		tty_write(str, strlen(str));
}
