#ifndef SYSCALL_H
# define SYSCALL_H

# include <memory/memory.h>
# include <process/process.h>

# include <libc/errno.h>
# include <libc/string.h>

# define PROT_READ		0b001
# define PROT_WRITE		0b010
# define PROT_EXEC		0b100
# define PROT_NONE		0b000

# define MAP_PRIVATE	0b00
# define MAP_SHARED		0b01
# define MAP_FIXED		0b10

typedef int32_t sys_ret_t;
typedef sys_ret_t (*sys_handler_t)(process_t *, const regs_t *);

sys_ret_t syscall_handler(const regs_t *registers);

sys_ret_t sys_write(process_t *process, const regs_t *registers);
sys_ret_t sys_fork(process_t *process, const regs_t *registers);
sys_ret_t sys_exit(process_t *process, const regs_t *registers);
sys_ret_t sys_getpid(process_t *process, const regs_t *registers);
sys_ret_t sys_getppid(process_t *process, const regs_t *registers);
sys_ret_t sys_waitpid(process_t *process, const regs_t *registers);
sys_ret_t sys_mmap(process_t *process, const regs_t *registers);
sys_ret_t sys_munmap(process_t *process, const regs_t *registers);
// TODO

#endif
