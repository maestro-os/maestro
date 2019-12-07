#include <process/process.h>

int write(int fildes, const void *buf, size_t nbyte);
pid_t fork(void);
void _exit(int status);
pid_t getpid(void);
pid_t getppid(void);
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

	putstr("fork\n");
	if((pid = fork()) < 0)
	{
		putstr("END\n");
		_exit(1);
	}
	if(pid == 0)
		putstr("child\n");
	else
	{
		putnbr(pid);
		putstr("parent\n");
	}
	fork_bomb();
}*/

void multi_fork(int count)
{
	pid_t pid;

loop:
	if(count == 0)
		return;
	if((pid = fork()) < 0)
		putstr("err\n");
	if(pid)
	{
		--count;
		goto loop;
	}
	putstr("child pid: ");
	putnbr(getpid());
	putstr("\n");
}

void test_process(void)
{
	//pid_t pid;

	putstr("pid: ");
	putnbr(getpid());
	putstr("\n");
	multi_fork(2);
	//fork_bomb();
	putstr("test_process end\n");
	*((char *) 0x0) = 0;
	while(1)
		;
	asm("hlt");
}
