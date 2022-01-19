#include <stddef.h>

typedef void (*sighandler_t)(int);

int open(char *pathname, int flags);
size_t read(int fd, void *buf, size_t count);
size_t write(int fd, const void *buf, size_t count);
void close(int status);
void _exit(int status);
int fork(void);
int waitpid(int pid, int *wstatus, int options);
int getpid(void);
int getppid(void);
void *mmap(void *addr, size_t len, int prot, int flags, int fildes, size_t off);
void munmap(void *addr, size_t length);
sighandler_t signal(int signum, sighandler_t handler);
int kill(int pid, int sig);
int socketpair(int domain, int type, int protocol, int vs[2]);

int init_module(void *module_image, size_t len);
int finit_module(int fd);
int delete_module(char *name);

void print_nbr(int nbr)
{
	if (nbr < 0)
		write(1, "-", 1);
	if (nbr >= 10) {
		print_nbr(nbr / 10);
	}
	if (nbr <= -10) {
		print_nbr(-(nbr / 10));
	}

	char c;
	if (nbr >= 0) {
		c = '0' + (nbr % 10);
	} else {
		c = '0' + (-(nbr % 10));
	}
	write(1, &c, 1);
}

void sig_handle(int sig) {
	(void) sig;

	write(1, ":(\n", 3);
	//*((int *) 0x0) = 42;
}

void test_process(void)
{
	// 42

	//write(1, "42", 2);
	//while(1)
	//	;



	// Testing printing on standard output

	//for(size_t i = 0; i < 10; ++i) {
	//	write(1, "pid: ", 5);
	//	print_nbr(getpid());
	//	write(1, "\n", 1);
	//}



	// Fork bomb

	while (1)
	{
		print_nbr(getpid());
		fork();
	}



	// Testing wait and signals

	//write(1, "Hello world!\n", 13);
	//int pid = fork();
	//if (pid == 0) {
	//	write(1, "forked!\n", 8);

	//	signal(1, sig_handle);
	//	kill(getpid(), 1);

	//	int pid2 = fork();
	//	if (pid2 == 0) {
	//		while(1)
	//			;
	//	}

	//	kill(pid2, 1);

	//	_exit(43);
	//} else {
	//	write(1, "waiting\n", 8);
	//	int wstatus = 42;
	//	int ret = waitpid(-1, &wstatus, 0);

	//	write(1, "ret: ", 5);
	//	print_nbr(ret);
	//	write(1, "\nstatus: ", 9);
	//	print_nbr(wstatus);

	//	while (1)
	//		;
	//}



	// Testing stop signals

	//int pid = fork();
	//if (pid == 0) {
	//	kill(getpid(), 14);

	//	for (int i = 0; i < 100; ++i)
	//		write(1, "b", 1);
	//} else {
	//	for (int i = 0; i < 100; ++i)
	//		write(1, "a", 1);

	//	kill(pid, 5);

	//	// If uncommented, success
	//	print_nbr(waitpid(-1, 0, 0));

	//	write(1, "noice", 5);

	//	// If uncommented, success
	//	print_nbr(waitpid(-1, 0, 0));

	//	write(1, "nOiCe", 5);

	//	// If uncommented, returns ECHILD
	//	print_nbr(waitpid(-1, 0, 0));

	//	write(1, "NOICE", 5);

	//	while (1)
	//		;
	//}



	// Testing IPC

	//int socks[2];
	//int e = socketpair(0, 0, 0, socks);
	//write(1, "e: ", 3);
	//print_nbr(e);
	//write(1, "\n", 1);

	//int pid = fork();
	//if (pid == 0) {
	//	//for (int i = 0; i < 100; ++i)
	//	while (1)
	//		write(socks[0], "BLEH", 4);
	//} else {
	//	while (1) {
	//		char buff[10];

	//		int len = read(socks[1], buff, sizeof(buff));
	//		if (len >= 0) {
	//			write(1, buff, len);
	//		}
	//	}
	//}



	// Testing mmap/munmap

	//size_t len = 100 * 4096;
	//char *ptr = mmap(NULL, len, 0b111, 0, 0, 0);

	//fork();

	//for(size_t i = 0; i < 26; ++i)
	//	ptr[i] = 'a' + i;
	//write(1, ptr, 26);

	//munmap(ptr, len);
	////for(size_t i = 0; i < 26; ++i)
	////	ptr[i] = 'a' + i;
	////write(1, ptr, 26);

	//while(1)
	//	;



	// Testing file read/write

	//int fd = open("/etc/hostname", 0b11);
	//char buff[1024];
	//if (fd < 0) {
	//	write(1, "Error\n", 6);
	//} else {
	//	int len = read(fd, buff, sizeof(buff));
	//	write(1, "len:", 5);
	//	print_nbr(len);
	//	write(1, "\n", 1);

	//	write(1, "Content:\n", 9);
	//	write(1, buff, len);
	//	write(1, "end\n", 4);

	//	// ------------------------------

	//	//buff[0] = 'A';
	//	//buff[1] = 'B';
	//	//buff[2] = 'C';
	//	//len = write(fd, buff, 3);
	//	//write(1, "len:", 5);
	//	//print_nbr(len);
	//	//write(1, "\n", 1);
	//}

	//while(1)
	//	;



	// Testing kernel module loading from disk

	//int fd = open("/lib/e1000.kmod", 0b11);
	//if (fd < 0) {
	//	write(1, "Error\n", 6);
	//} else {
	//	print_nbr(finit_module(fd));
	//}

	while(1)
		;
	asm("hlt");
}
