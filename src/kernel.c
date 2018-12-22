#include "kernel.h"
#include "tty/tty.h"
#include "vga/vga.h"

void kernel_main()
{
	// TODO
	vga_clear();

	const char* str = "Hello world!";
	for(size_t i = 0; i < 10; ++i) tty_write(str, strlen(str));
}
