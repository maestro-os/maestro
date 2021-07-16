#include <stddef.h>

void write(int fd, const void *buf, size_t count);
void close(int status);
void _exit(int status);
int fork(void);
int waitpid(int pid, int *wstatus, int options);
int getpid(void);
int getppid(void);

void print_nbr(int nbr)
{
	if (nbr < 0)
		write(1, "-", 1);
	if (nbr >= 10) {
		print_nbr(nbr / 10);
	}
	if (-nbr >= 10) {
		print_nbr(-nbr / 10);
	}

	char c;
	if (nbr >= 0) {
		c = '0' + (nbr % 10);
	} else {
		c = '0' + (-nbr % 10);
	}
	write(1, &c, 1);
}

void test_process(void)
{
	//for(size_t i = 0; i < 10; ++i) {
	//	write(1, "pid: ", 5);
	//	print_nbr(getpid());
	//	write(1, "\n", 1);
	//}

	//while (1)
	//{
	//	print_nbr(getpid());
	//	fork();
	//}

	write(1, "Hello world!\n", 13);
	int pid = fork();
	if (pid == 0) {
		write(1, "forked!\n", 8);
		_exit(42);
	} else {
		write(1, "waiting\n", 8);
		int wstatus = 42;
		int ret = waitpid(-1, &wstatus, 0);

		write(1, "ret: ", 5);
		print_nbr(ret);
		write(1, "\nstatus: ", 9);
		print_nbr(wstatus);

		while (1)
			;
	}
	asm("hlt");
}
