#ifndef SYSCALL_H
# define SYSCALL_H

# include <memory/memory.h>
# include <process/process.h>

# include <libc/errno.h>
# include <libc/string.h>
# include <libc/sys/types.h>

# define TO_PTR(val)	((void *) (val))

typedef int32_t sys_ret_t;
typedef sys_ret_t (*sys_handler_t)(process_t *, const regs_t *);

sys_ret_t syscall_handler(const regs_t *registers);

sys_ret_t sys_write(process_t *process, const regs_t *registers);
sys_ret_t sys_fork(process_t *process, const regs_t *registers);
sys_ret_t sys_exit(process_t *process, const regs_t *registers);
sys_ret_t sys_getpid(process_t *process, const regs_t *registers);
sys_ret_t sys_getppid(process_t *process, const regs_t *registers);
sys_ret_t sys_waitpid(process_t *process, const regs_t *registers);
// TODO

#endif
