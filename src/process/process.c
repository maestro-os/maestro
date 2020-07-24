#include <kernel.h>
#include <libc/errno.h>
#include <memory/slab/slab.h>
#include <process/process.h>
#include <process/scheduler.h>
#include <util/util.h>
#include <debug/debug.h>

#define USER_STACK_FLAGS\
	MEM_REGION_FLAG_STACK | MEM_REGION_FLAG_WRITE | MEM_REGION_FLAG_USER
#define KERNEL_STACK_FLAGS\
	MEM_REGION_FLAG_STACK | MEM_REGION_FLAG_WRITE

#define USER_STACK_PAGES	8
#define KERNEL_STACK_ORDER	3

// TODO Documentation and cleanup
// TODO Set errnos
// TODO Multicore handling

/*
 * The cache for processes structures.
 */
static cache_t *processes_cache;

/*
 * The cache for signals structures.
 */
static cache_t *signals_cache;

/*
 * Processes list.
 */
avl_tree_t *processes = NULL;

/*
 * The currently running process.
 */
process_t *running_process = NULL;

/*
 * The bitfield indicating which PIDs are used.
 */
static uint8_t *pids_bitfield;

/*
 * The Task State Segment structure used to specify the pointer to the syscall
 * stack.
 */
ATTR_PAGE_ALIGNED
ATTR_BSS
tss_entry_t tss;

/*
 * The list of interrupt stacks for every cores.
 */
void **interrupt_stacks;

/*
 * The spinlock for processes handling.
 */
spinlock_t processes_spinlock = 0;

/*
 * Constructs a process structure.
 */
ATTR_HOT
static void process_ctor(void *ptr, size_t size)
{
	process_t *p;
	size_t i = 0;

	bzero(ptr, size);
	p = ptr;
	if(WAITING != 0)
	{
		p->state = WAITING;
		p->prev_state = WAITING;
	}
	while(i < SIG_MAX)
		p->sigactions[i++].sa_handler = SIG_DFL;
}

/*
 * Destructs a process structure.
 */
ATTR_HOT
static void process_dtor(void *ptr, size_t size)
{
	process_t *p;
	list_head_t *c;

	p = ptr;
	(void) size;
	avl_tree_remove(&processes, &p->tree);
	scheduler_remove(p);
	if(p->parent)
		list_remove(&p->parent->children, &p->parent_child);
	c = p->children;
	while(c)
	{
		list_remove(&p->children, c->prev);
		CONTAINER_OF(c, process_t, parent_child)->parent = NULL;
		c = c->next;
	}
	if(p->sem_curr)
		sem_remove(p->sem_curr, p);
	mem_space_destroy(p->mem_space);
	// TODO Empty signals queue
}

/*
 * Initializes the TSS into the Global Descriptor Table and updates it.
 */
ATTR_COLD
static void tss_init(void)
{
	const uint32_t base = (uint32_t) &tss;
	const unsigned limit = sizeof(tss_entry_t);
	const uint8_t flags = 0b0100;
	const uint8_t access = 0b10001001;
	gdt_entry_t *tss_gdt;

	tss_gdt = tss_gdt_entry();
	bzero(tss_gdt, sizeof(gdt_entry_t));
	tss_gdt->limit_low = limit & 0xffff;
	tss_gdt->base_low = base & 0xffff;
	tss_gdt->base_mid = (base >> 16) & 0xff;
	tss_gdt->access = access;
	tss_gdt->flags_limit = ((limit >> 16) & 0xf) | (flags << 4);
	tss_gdt->base_high = (base >> 24) & 0xff;

	bzero(&tss, sizeof(tss_entry_t));
	tss_flush();
}

/*
 * Allocates an interrupt stack for every cores of the CPU and initializes the
 * TSS for each.
 */
ATTR_COLD
static void interrupt_stacks_init(void)
{
	// TODO Multiple core
	const size_t cores_count = 1;
	size_t i;

	if(!(interrupt_stacks = kmalloc(sizeof(void *) * cores_count)))
		PANIC("Cannot initialize interrupt stacks!", 0);
	for(i = 0; i < cores_count; ++i)
	{
		if(!(interrupt_stacks[i] = buddy_alloc(INTERRUPT_STACK_ORDER)))
			PANIC("Cannot initialize interrupt stack!", 0);
		tss_init();
		tss.ss0 = GDT_KERNEL_DATA_OFFSET;
		tss.ss = GDT_USER_DATA_OFFSET;
		tss.esp0 = (uint32_t) interrupt_stacks[i];
	}
}

/*
 * Initializes caches for processes, PIDs bitmap, interrupt stacks and TSS.
 */
ATTR_COLD
void process_init(void)
{
	processes_cache = cache_create("processes", sizeof(process_t), 64,
		process_ctor, process_dtor);
	signals_cache = cache_create("signals", sizeof(signal_t), 64,
		NULL, bzero);
	if(!processes_cache || !signals_cache)
		PANIC("Cannot allocate caches for processes!", 0);
	if(!(pids_bitfield = kmalloc_zero(PIDS_BITFIELD_SIZE)))
		PANIC("Cannot allocate PIDs bitfield!", 0);
	bitfield_set(pids_bitfield, 0);
	interrupt_stacks_init();
}

/*
 * Allocates a PID and returns it. Returns `-1` if no PID is available.
 */
ATTR_HOT
static pid_t alloc_pid(void)
{
	pid_t pid;

	// TODO Use a last_pid variable to avoid searching from the first pid
	pid = bitfield_first_clear(pids_bitfield, PIDS_BITFIELD_SIZE);
	if(pid >= (pid_t) PIDS_BITFIELD_SIZE)
		return -1;
	bitfield_set(pids_bitfield, pid);
	return pid;
}

/*
 * Frees the given PID into the bitmap.
 */
ATTR_HOT
static void free_pid(const pid_t pid)
{
	bitfield_clear(pids_bitfield, pid);
}

/*
 * Creates a new process with its own PID. `parent` is the parent of the newly
 * created process. `registers` is the initial states of the registers for the
 * process.
 *
 * If `parent` is not NULL, the parent's memory space is cloned for the new
 * process. User stacks are cloned but not kernel stacks.
 *
 * The process is added as a child to `parent` and is added to the processes
 * list.
 */
ATTR_HOT
process_t *process_create(process_t *parent, const regs_t *registers)
{
	pid_t pid;
	process_t *new_proc;

	spin_lock(&processes_spinlock);
	errno = 0;
	if((pid = alloc_pid()) < 0
		|| !(new_proc = cache_alloc(processes_cache)))
	{
		errno = ENOMEM;
		goto fail;
	}
	new_proc->pid = pid;
	new_proc->parent = parent;
	new_proc->regs_state = *registers;
	debug_assert(new_proc->state == WAITING,
		"process: invalid state for new process");
	scheduler_add(new_proc);
	if(!parent)
	{
		if(!(new_proc->mem_space = mem_space_init()))
			goto fail;
		if(!(new_proc->user_stack = mem_space_alloc(new_proc->mem_space,
			USER_STACK_PAGES, USER_STACK_FLAGS)))
			goto fail;
	}
	else
	{
		if(!(new_proc->mem_space = mem_space_clone(parent->mem_space)))
			goto fail;
		new_proc->user_stack = parent->user_stack;
	}
	if(!(new_proc->kernel_stack
		= mem_space_alloc_kernel_stack(new_proc->mem_space,
			KERNEL_STACK_ORDER)))
		goto fail;
	if(!parent)
		new_proc->regs_state.esp = (uintptr_t) new_proc->user_stack;
	process_add_child(parent, new_proc);
	if(errno)
		goto fail;
	new_proc->tree.value = new_proc->pid;
	avl_tree_insert(&processes, &new_proc->tree, avl_val_cmp);
	spin_unlock(&processes_spinlock);
	return new_proc;

fail:
	free_pid(pid);
	// TODO Free all
	spin_unlock(&processes_spinlock);
	return NULL;
}

/*
 * Returns the process with the given PID. If the process doesn't exist, the
 * function returns NULL.
 */
ATTR_HOT
process_t *process_get(const pid_t pid)
{
	avl_tree_t *n;
	process_t *p;

	debug_assert(pid < PID_MAX, "process: bad PID");
	spin_lock(&processes_spinlock);
	n = avl_tree_search(processes, pid, avl_val_cmp);
	if(!n)
	{
		errno = ESRCH;
		return NULL;
	}
	p = CONTAINER_OF(n, process_t, tree);
	spin_unlock(&processes_spinlock);
	return p; // TODO Warning: process might get invalid since execution is out of spinlock
}

/*
 * Sets the state `state` for the given process `process` and update the prevous
 * state.
 * If state is `RUNNING`, the currently running process is updated.
 * If state is `TERMINATED` and the process is waiting into a semaphore, then
 * it is removed from said semaphore it.
 */
ATTR_HOT
void process_set_state(process_t *process, const process_state_t state)
{
	if(!process)
		return;
	spin_lock(&processes_spinlock);
	// TODO Ignore if state isn't changed
	if(state == RUNNING)
	{
		if(running_process)
		{
			running_process->prev_state = running_process->state;
			running_process->state = WAITING;
		}
		running_process = process;
	}
	else if(process == running_process)
		running_process = NULL;
	process->prev_state = process->state;
	process->state = state;
	if(state == WAITING)
		scheduler_add(process);
	else
		scheduler_remove(process);
	if(state == TERMINATED)
		sem_remove(process->sem_curr, process);
	spin_unlock(&processes_spinlock);
}

/*
 * Adds child `child` to the given parent process `parent`.
 */
ATTR_HOT
void process_add_child(process_t *parent, process_t *child)
{
	if(!sanity_check(parent) || !sanity_check(child))
		return;
	spin_lock(&parent->spinlock); // TODO Also aquire child's spinlock
	list_insert_front(&parent->children, &child->parent_child);
	spin_unlock(&parent->spinlock);
}

/*
 * Makes the given process `process` exit with status `status`, changing the
 * state of the process to `TERMINATED`.
 */
ATTR_HOT
void process_exit(process_t *process, int status)
{
	if(!process)
		return;
	spin_lock(&process->spinlock);
	process->status = status;
	process_set_state(process, TERMINATED);
	spin_unlock(&process->spinlock);
}

// TODO Limit on signals?
// TODO Perform signals directly?
// TODO Execute signal later?
// TODO Send signals to children
/*
 * Kills the given process `process` with the given signal number `sig`.
 */
ATTR_HOT
void process_kill(process_t *process, int sig)
{
	signal_t *s;
	sigaction_t *action;

	if(!sanity_check(process))
		return;
	spin_lock(&process->spinlock);
	action = &process->sigactions[sig];
	if(sig == SIGKILL || sig == SIGSTOP || action->sa_handler == SIG_DFL)
	{
		signal_default(process, sig);
		goto end;
	}
	if(action->sa_handler == SIG_IGN)
		goto end;
	if(!(s = cache_alloc(signals_cache)))
	{
		// TODO Error
		goto end;
	}
	s->info.si_signo = sig;
	// TODO
	queue_enqueue(&process->last_signal, &process->signals_queue, &s->queue);

end:
	spin_unlock(&process->spinlock);
}
