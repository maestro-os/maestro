#include "kernel.h"
#include "tty/tty.h"
#include "vga/vga.h"

void kernel_main(const void* boot_infos)
{
	(void) boot_infos;

	// TODO
	vga_clear();

	const char* str = "Hello world!";
	for(size_t i = 0; i < 10; ++i) tty_write(str, strlen(str));
}
