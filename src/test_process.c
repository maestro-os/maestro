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
	//for(size_t i = 0; i < 10; ++i) {
	//	write(1, "pid: ", 5);
	//	print_nbr(getpid());
	//	write(1, "\n", 1);
	//}

	fork();
	if(getpid() == 1)
	{
		write(1, "1:1\n", 4);
	}
	else
	{
		write(1, "1:2\n", 4);
	}
	fork();
	switch(getpid()) {
		case 1:
			write(1, "2:1\n", 4);
			break;
		case 2:
			write(1, "2:2\n", 4);
			break;
		case 3:
			write(1, "2:3\n", 4);
			break;
		case 4:
			write(1, "2:4\n", 4);
			break;
	}
	asm("hlt");
}
