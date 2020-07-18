#ifndef PROCESS_H
# define PROCESS_H

# include <gdt.h>
# include <memory/mem_space/mem_space.h>
# include <process/tss.h>
# include <process/signal.h>

/*
 * The maximum number of PIDs.
 */
# define PID_MAX			32768
/*
 * The size of the bitfield containing PID allocations.
 */
# define PIDS_BITFIELD_SIZE	BITFIELD_SIZE(PID_MAX)

/*
 * Structure representing the list of registers for a context.
 */
ATTR_PACKED
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

/*
 * States of a process.
 */
typedef enum
{
	/* The process is paused, waiting for its turn to be executed */
	WAITING,
	/* The process is currently running */
	RUNNING,
	/* The process is waiting for a resource to be available */
	BLOCKED,
	/* The process has been paused and is waiting to be resumed */
	STOPPED,
	/*
	 * The process is terminated and is waiting for its status to be retrived
	 * before being deleted
	 */
	TERMINATED
} process_state_t;

/*
 * A semaphore.
 */
typedef struct
{
	/* The process currently using the resource */
	process_t *proc_current;

	/*
	 * The next process to aquire the resource, last element of waiting
	 * processes list
	 */
	list_head_t *proc_queue;
	/* The list of waiting processes */
	list_head_t *proc_queue_last;

	/* The spinlock */
	spinlock_t spinlock;
} semaphore_t;

/*
 * Structure representing a process.
 */
typedef struct process
{
	/* The tree storing all processes by PID */
	avl_tree_t tree;

	/* The PID of the process. */
	pid_t pid;
	/* The user id of the owner of the process. */
	uid_t owner_id;
	/* The current and previous state of the process. */
	process_state_t state, prev_state;

	/* The priority of the process. */
	int8_t priority;
	/* The scheduler queue. */
	list_head_t schedule_queue;

	/* A pointer to the parent process. */
	struct process *parent;
	/* The linked list of children of the parent process. */
	list_head_t parent_child;
	/*
	 * A pointer to the first element of the linked list of the processes'
	 * children.
	 */
	list_head_t *children;

	/*
	 * A pointer to the current semaphore in which the process is waiting.
	 * NULL if the process is not BLOCKED.
	 */
	semaphore_t *sem_curr;
	/* The queue for the semaphore. */
	list_head_t sem_queue;

	/* The memory space for the process */
	mem_space_t *mem_space;
	/* The pointer to the userspace stack */
	void *user_stack;
	/* The pointer to the kernelspace stack */
	void *kernel_stack;
	// TODO Add signals stack?
	/* The saved state of the process's context registers */
	regs_t regs_state;
	/*
	 * Tells whether the process was executing a syscall or not before being
	 * interrupted
	 */
	char syscalling;

	/* The array of actions for every signals. */
	sigaction_t sigactions[SIG_MAX];
	/* The signals queue */
	queue_head_t *signals_queue, *last_signal;
	/*
	 * The exit status of the process to be retrieved when state is TERMINATED.
	 */
	int status;

	/* The spinlock for the process's structure */
	spinlock_t spinlock;
} process_t;

void sem_init(semaphore_t *sem);
void sem_wait(semaphore_t *sem, process_t *process);
void sem_remove(semaphore_t *sem, process_t *process);
void sem_post(semaphore_t *sem);

extern gdt_entry_t *tss_gdt_entry(void);
extern void tss_flush(void);

void process_init(void);
process_t *process_create(process_t *parent, const regs_t *registers);
process_t *process_get(pid_t pid);
void process_set_state(process_t *process, process_state_t state);
void process_add_child(process_t *parent, process_t *child);
void process_exit(process_t *process, int status);
void process_kill(process_t *process, int sig);

#endif
