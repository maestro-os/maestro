#ifndef PROCESS_H
# define PROCESS_H

# include <gdt/gdt.h>
# include <memory/memory.h>

# include <libc/sys/types.h>
# include <stdint.h>

# define PID_MAX			32768
# define PIDS_BITMAP_SIZE	(PID_MAX / BIT_SIZEOF(char))

# define SIGHUP		1
# define SIGINT		2
# define SIGQUIT	3
# define SIGILL		4
# define SIGTRAP	5
# define SIGABRT	6
# define SIGBUS		7
# define SIGFPE		8
# define SIGKILL	9
# define SIGUSR1	10
# define SIGSEGV	11
# define SIGUSR2	12
# define SIGPIPE	13
# define SIGALRM	14
# define SIGTERM	15
# define SIGCHLD	17
# define SIGCONT	18
# define SIGSTOP	19
# define SIGTSTP	20
# define SIGTTIN	21
# define SIGTTOU	22
# define SIGURG		23
# define SIGXCPU	24
# define SIGXFSZ	25
# define SIGVTALRM	26
# define SIGPROF	27
# define SIGPOLL	29
# define SIGSYS		31

__attribute__((packed))
struct tss_entry
{
	uint32_t prev_tss;
	uint32_t esp0;
	uint32_t ss0;
	uint32_t esp1;
	uint32_t ss1;
	uint32_t esp2;
	uint32_t ss2;
	uint32_t cr3;
	uint32_t eip;
	uint32_t eflags;
	uint32_t eax;
	uint32_t ecx;
	uint32_t edx;
	uint32_t ebx;
	uint32_t esp;
	uint32_t ebp;
	uint32_t esi;
	uint32_t edi;
	uint32_t es;
	uint32_t cs;
	uint32_t ss;
	uint32_t ds;
	uint32_t fs;
	uint32_t gs;
	uint32_t ldt;
	uint16_t trap;
	uint16_t iomap_base;
};

typedef struct tss_entry tss_entry_t;

typedef enum
{
	CREATED,
	WAITING,
	RUNNING,
	BLOCKED,
	TERMINATED
} process_state_t;

typedef struct child child_t;

typedef struct signal
{
	struct signal *next;

	int si_signo;
	int si_code;
	int si_errno;

	pid_t si_pid;
	uid_t si_uid;
	void *si_addr;
	int si_status;

	long si_band;

	// TODO si_value
} signal_t;

typedef struct process
{
	struct process *next;

	pid_t pid;
	process_state_t state;
	uid_t owner_id;

	struct process *parent;
	child_t *children;

	vmem_t page_dir;
	void *user_stack;
	void *kernel_stack;
	tss_entry_t tss;

	signal_t *signals_queue;

	void (*begin)();
} process_t;

struct child
{
	struct child *next;
	process_t *process;
};

extern gdt_entry_t *tss_gdt_entry(void);
extern void tss_flush(void);

void process_init(void);
process_t *new_process(process_t *parent, void (*begin)());
process_t *get_process(const pid_t pid);
process_t *get_running_process(void);
process_t *process_clone(process_t *proc);
void process_add_child(process_t *parent, process_t *child);
void del_process(process_t *process, const bool children);

void process_tick(void);
extern void context_switch(void *esp, void *eip);

#endif
