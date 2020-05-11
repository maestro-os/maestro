#include <process/process.h>
#include <syscall/syscall.h>

typedef unsigned off_t;

int write(int fildes, const void *buf, size_t nbyte);
pid_t fork(void);
void _exit(int status);
pid_t getpid(void);
pid_t getppid(void);
pid_t waitpid(pid_t pid, int *wstatus, int options);
void *mmap(void *addr, size_t length, int prot, int flags,
	int fd, off_t offset);
int munmap(void *addr, size_t length);

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
	{
		putnbr(getpid());
		fork();
	}*/
	/*while(1)
		putnbr(getpid());*/
	/*putstr("fork: ");
	putnbr(fork());
	putstr("\n");*/

	putstr("pid: ");
	putnbr(getpid());
	putstr("\n");

	/*
	char *ptr = mmap(NULL, 0x1001, PROT_READ | PROT_WRITE, MAP_PRIVATE, -1, 0);
	size_t i = 0;
	while(i < 0x1001)
		ptr[i++] = 0xff;
	putstr("still alive\n");*/

	while(1)
	{
		char *ptr;
		if(!(ptr = mmap(NULL, 0x1000, PROT_READ | PROT_WRITE, MAP_PRIVATE,
			-1, 0)))
		{
			putstr("NULL\n");
			break;
		}
		ptr[0] = 0xff;
	}

	while(1)
		;
	asm("hlt");
}
