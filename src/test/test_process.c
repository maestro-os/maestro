#include <process/process.h>

int write(int fildes, const void *buf, size_t nbyte);
pid_t fork(void);
void _exit(int status);
pid_t getpid(void);
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

static void putstr(const char *s)
{
	write(0, s, strlen(s));
}

/*static void fork_bomb(void)
{
	pid_t pid;
	int status;

	write(0, "fork\n", 5);
	if((pid = fork()) < 0)
	{
		putnbr(-pid);
		putstr("END\n");
		_exit(1);
		return;
	}
	if(pid == 0)
	{
		putstr("child\n");
		fork_bomb();
	}
	else
	{
		putnbr(pid);
		putstr("parent\n");
		waitpid(pid, &status, 0);
		_exit(status); // TODO EXITSTATUS
	}
}*/

void test_process(void)
{
	pid_t pid;

	/*write(0, "BEGIN\n", 6);
	fork_bomb();*/
	if((pid = getpid()) == 1)
		*((char *)0x0) = 42;
	while(1)
	{
		putstr("pid: ");
		putnbr(pid);
		putchar('\n');
	}
	while(1)
		;
	asm("hlt");
}
