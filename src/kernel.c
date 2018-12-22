#include "kernel.h"
#include "tty/tty.h"
#include "vga/vga.h"

void kernel_main(const void* bi)
{
	const boot_info_t boot_info = get_boot_info(bi);
	(void) boot_info;

	vga_clear();

	const char* str = "Hello world!";
	for(size_t i = 0; i < 10; ++i) tty_write(str, strlen(str));
}
