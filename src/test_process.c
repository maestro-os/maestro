#include <stddef.h>

void write(int fd, const void *buf, size_t count);
void close(int status);
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
	write(1, &c, 1);
}

void test_process(void)
{
	for(size_t i = 0; i < 10; ++i) {
		write(1, "pid: ", 5);
		print_nbr(getpid());
		write(1, "\n", 1);
	}

	//fork();
	//fork();
	while(1)
		;
		//print_nbr(getpid());
	asm("hlt");
}
