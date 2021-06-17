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

	int pid = fork();
	if(pid) {
		//write(1, "parent pid: ", 12);
		//print_nbr(pid);
	} else {
		//write(1, "child pid: ", 11);
		//print_nbr(getpid());
	}

	while(1)
		;
	asm("hlt");
}
