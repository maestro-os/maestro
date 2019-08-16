#ifndef SYSCALL_H
# define SYSCALL_H

# include <memory/memory.h>
# include <process/process.h>

# include <libc/errno.h>
# include <libc/string.h>
# include <libc/sys/types.h>

# define TO_PTR(val)	((void *) (val))

__attribute__((packed))
struct sys_info
{
	int32_t eax;
	int32_t ebx;
	int32_t ecx;
	int32_t edx;
	int32_t esi;
	int32_t edi;
	int32_t ebp;
};

typedef struct sys_info sys_info_t;
typedef int32_t sys_ret_t;
typedef sys_ret_t (*sys_handler_t)(process_t *, const sys_info_t *);

sys_ret_t syscall_handler(const sys_info_t *info);

sys_ret_t sys_write(process_t *process, const sys_info_t *info);
sys_ret_t sys_fork(process_t *process, const sys_info_t *info);
sys_ret_t sys_exit(process_t *process, const sys_info_t *info);
sys_ret_t sys_getpid(process_t *process, const sys_info_t *info);
sys_ret_t sys_waitpid(process_t *process, const sys_info_t *info);
// TODO

#endif
