#include "kernel.h"
#include "tty/tty.h"
#include "vga/vga.h"

void kernel_main(const void* bi)
{
	(void) bi;
	tty_init();

	// boot_info = load_boot_info(bi);

	//const char* str = "Hello world!";
	//for(size_t i = 0; i < 10; ++i) tty_write(str, strlen(str));
	vga_putchar('H', 0, 0);
}
