#ifndef SYSCALL_H
# define SYSCALL_H

# include <memory/memory.h>

# include <libc/errno.h>
# include <libc/string.h>
# include <libc/sys/types.h>

# define MAP_FAILED	((void *) -1)

// TODO

void *mmap(void *addr, size_t length, int prot, int flags,
	int fd, off_t offset);
int munmap(void *addr, size_t length);

#endif
