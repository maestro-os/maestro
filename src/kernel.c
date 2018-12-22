#include "kernel.h"

void kernel_main()
{
	// TODO
	vga_init();
	vga_putstr("Hello world!", 0, 0);
}
