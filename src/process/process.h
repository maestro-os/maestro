#ifndef PROCESS_H
# define PROCESS_H

# include <kernel.h>
# include <memory/memory.h>
# include <process/tss.h>
# include <process/signal.h>

# define PID_MAX			32768
# define PIDS_BITMAP_SIZE	(PID_MAX / BIT_SIZEOF(char))

__attribute__((packed))
struct regs
{
	int32_t ebp;
	int32_t esp;
	int32_t eip;
	int32_t eflags;
	int32_t eax;
	int32_t ebx;
	int32_t ecx;
	int32_t edx;
	int32_t esi;
	int32_t edi;
};

typedef struct regs regs_t;

typedef enum
{
	CREATED,
	WAITING,
	RUNNING,
	BLOCKED,
	STOPPED,
	TERMINATED
} process_state_t;

typedef struct child child_t;

typedef struct process
{
	struct process *next;

	pid_t pid;
	process_state_t state, prev_state;
	uid_t owner_id;

	struct process *parent;
	child_t *children;

	vmem_t page_dir;
	void *user_stack;
	void *kernel_stack;
	regs_t regs_state;
	bool syscalling;

	sigaction_t sigactions[SIG_MAX];
	signal_t *signals_queue, *last_signal;
	int status;

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
void process_set_state(process_t *process, process_state_t state);
void process_add_child(process_t *parent, process_t *child);
void process_exit(process_t *proc, int status);
void process_kill(process_t *proc, int sig);
void del_process(process_t *process, const bool children);

void process_tick(const regs_t *registers);

__attribute__((noreturn))
extern void context_switch(void *esp, void *eip,
	uint16_t data_selector, uint16_t code_selector, vmem_t vmem);
__attribute__((noreturn))
extern void kernel_switch(const regs_t *regs,
	uint16_t data_selector, uint16_t code_selector);

#endif
