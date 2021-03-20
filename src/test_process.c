// TODO doc?

#include <stddef.h>

void write(int fd, const void *buf, size_t count);
void _exit(int status);

// TODO doc?
void test_process(void)
{
	while(1)
		write(0, "Hello world!\n", 13);
	asm("hlt");
}
