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

void fork_bomb(void)
{
	while(1)
	{
		if(fork() < 0)
			putstr("fork error\n");
		putstr("fork");
	}
}

void multi_branch_fork(int count)
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

void multi_chain_fork(int count)
{
	pid_t pid;

loop:
	if(count == 0)
		return;
	if((pid = fork()) < 0)
		putstr("err\n");
	if(pid)
		return;
	--count;
	goto loop;
}

void test_process(void)
{
	//multi_branch_fork(10);
	/*while(1)
		fork();*/
	while(1)
		putnbr(getpid());
	asm("hlt");
}
