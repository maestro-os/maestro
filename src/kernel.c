#include "kernel.h"
#include "tty/tty.h"
#include "framebuffer/framebuffer.h"

void kernel_main(const void* bi)
{
	boot_info = load_boot_info(bi);

	vga_clear();

	const char* str = "Hello world!";
	for(size_t i = 0; i < 10; ++i) tty_write(str, strlen(str));
}
