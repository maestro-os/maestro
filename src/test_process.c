// TODO doc?

#include <stddef.h>

void write(int fd, const void *buf, size_t count);
void _exit(int status);
int fork(void);
int getpid(void);
int getppid(void);

void print_nbr(unsigned nbr)
{
	if(nbr >= 10) {
		print_nbr(nbr / 10);
	}

	const char c = '0' + (nbr % 10);
	write(0, &c, 1);
}

// TODO doc?
void test_process(void)
{
	for(size_t i = 0; i < 10; ++i) {
		write(0, "pid: ", 5);
		print_nbr(getpid());
		write(0, "\n", 1);
	}
	/*int pid = fork();
	if(pid) {
		// TODO
	} else {
		print_nbr(pid);
	}*/

	while(1)
		;
	asm("hlt");
}
