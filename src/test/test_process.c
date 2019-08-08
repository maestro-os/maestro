#include <process/process.h>

int write(int fildes, const void *buf, size_t nbyte);
pid_t fork(void);
void _exit(int status);
pid_t waitpid(pid_t pid, int *wstatus, int options);

static void putchar(char c)
{
	write(0, &c, 1);
}

static void putnbr(int n)
{
	if(n < 0)
	{
		putchar('-');
		n = -n;
	}
	if(n > 9)
		putnbr(n / 10);
	putchar('0' + (n % 10));
}

static void fork_bomb(void)
{
	pid_t pid;
	int status;

	write(0, "fork\n", 5);
	if((pid = fork()) < 0)
	{
		putnbr(-pid);
		write(0, "END\n", 4);
		_exit(1);
		return;
	}
	if(pid == 0)
		fork_bomb();
	else
	{
		waitpid(pid, &status, 0);
		_exit(status); // TODO EXITSTATUS
	}
}

void test_process(void)
{
	write(0, "BEGIN\n", 6);
	fork_bomb();
	while(1)
		;
	// TODO Protect when returning (Triple fault)
}
