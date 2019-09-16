#include <process/process.h>
#include <libc/errno.h>

// TODO lock when doing something
// TODO Set errnos
// TODO Multicore handling

static cache_t *processes_cache;
static cache_t *children_cache;
static cache_t *signals_cache;

static process_t *processes = NULL;
static uint8_t *pids_bitmap;

__ATTR_PAGE_ALIGNED
__ATTR_BSS
static tss_entry_t tss;

static process_t *running_process = NULL;

__attribute__((hot))
static void process_ctor(void *ptr, const size_t size)
{
	process_t *p;
	size_t i = 0;

	bzero(ptr, size);
	p = ptr;
	if(CREATED != 0)
	{
		p->state = CREATED;
		p->prev_state = CREATED;
	}
	while(i < SIG_MAX)
		p->sigactions[i++].sa_handler = SIG_DFL;
}

__attribute__((cold))
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

__attribute__((cold))
void process_init(void)
{
	processes_cache = cache_create("processes", sizeof(process_t), PID_MAX,
		process_ctor, bzero);
	children_cache = cache_create("process_children", sizeof(child_t), PID_MAX,
		NULL, bzero);
	signals_cache = cache_create("signals", sizeof(siginfo_t), PID_MAX,
		NULL, bzero);
	if(!processes_cache || !children_cache || !signals_cache)
		PANIC("Cannot allocate caches for processes!", 0);

	if(!(pids_bitmap = kmalloc_zero(PIDS_BITMAP_SIZE, 0)))
		PANIC("Cannot allocate PIDs bitmap!", 0);
	bitmap_set(pids_bitmap, 0);

	tss_init();
}

__attribute__((hot))
static pid_t alloc_pid(void)
{
	pid_t pid;

	// TODO Use a last_pid variable to avoid searching from the first pid
	pid = bitmap_first_clear(pids_bitmap, PIDS_BITMAP_SIZE);
	if(pid >= (pid_t) PIDS_BITMAP_SIZE)
		return -1;
	bitmap_set(pids_bitmap, pid);
	return pid;
}

__attribute__((hot))
static void free_pid(const pid_t pid)
{
	bitmap_clear(pids_bitmap, pid);
}

// TODO Spinlock
__attribute__((hot))
process_t *new_process(process_t *parent, void (*begin)())
{
	pid_t pid;
	process_t *new_proc, *p;

	errno = 0;
	if((pid = alloc_pid()) < 0
		|| !(new_proc = cache_alloc(processes_cache)))
	{
		free_pid(pid);
		errno = ENOMEM;
		return NULL;
	}
	new_proc->pid = pid;
	new_proc->parent = parent;
	new_proc->begin = begin;
	process_add_child(parent, new_proc);
	if(errno)
	{
		// TODO Free all
		return NULL;
	}
	if(processes)
	{
		p = processes;
		while(p->next && p->next->pid < pid)
			p = p->next;
		new_proc->next = p->next;
		p->next = new_proc;
	}
	else
		processes = new_proc;
	return new_proc;
}

__attribute__((hot))
process_t *get_process(const pid_t pid)
{
	process_t *p;

	errno = 0;
	p = processes;
	if(pid <= 0)
	{
		errno = EINVAL;
		return NULL;
	}
	while(p)
	{
		if(p->pid == pid)
			return p;
		p = p->next;
	}
	errno = ESRCH;
	return NULL;
}

__attribute__((hot))
process_t *get_running_process(void)
{
	return running_process;
}

__attribute__((hot))
process_t *process_clone(process_t *proc)
{
	process_t *p;

	if(!proc)
	{
		errno = EINVAL;
		return NULL;
	}
	if(!(p = new_process(proc, (void *) proc->regs_state.eip)))
		return NULL;
	if(!(p->page_dir = vmem_clone(proc->page_dir, true)))
	{
		del_process(p, false);
		return NULL;
	}
	return p;
}

// TODO Pay attention to interrupts happening during this function? (setting to blocked during a syscall)
__attribute__((hot))
void process_set_state(process_t *process, const process_state_t state)
{
	if(!process)
		return;
	if(state == RUNNING)
	{
		if(running_process)
			process_set_state(running_process, WAITING);
		running_process = process;
	}
	else if((state == WAITING || state == BLOCKED)
		&& process == running_process)
		running_process = NULL;
	process->prev_state = process->state;
	process->state = state;
}

__attribute__((hot))
void process_add_child(process_t *parent, process_t *child)
{
	child_t *c;

	if(!parent || !child)
		return;
	if(!(c = cache_alloc(children_cache)))
	{
		errno = ENOMEM;
		return;
	}
	c->next = parent->children;
	c->process = child;
	parent->children = c;
}

__attribute__((hot))
void process_exit(process_t *proc, const int status)
{
	if(!proc)
		return;
	proc->status = status;
	process_set_state(proc, TERMINATED);
	if(running_process == proc)
		running_process = NULL;
}

// TODO Limit on signals?
// TODO Perform signals directly?
// TODO What if a signal is already being executed?
__attribute__((hot))
void process_kill(process_t *proc, const int sig)
{
	signal_t *s;
	sigaction_t *action;

	if(!proc)
		return;
	if(sig == SIGKILL || sig == SIGSTOP
		|| (action = proc->sigactions + sig)->sa_handler == SIG_DFL)
	{
		signal_default(proc, sig);
		return;
	}
	if(action->sa_handler == SIG_IGN)
		return;
	if(!(s = cache_alloc(signals_cache)))
		return;
	s->info.si_signo = sig;
	// TODO
	if(proc->last_signal)
	{
		proc->last_signal->next = s;
		proc->last_signal = s;
	}
	else
	{
		proc->signals_queue = s;
		proc->last_signal = s;
	}
}

__attribute__((hot))
void del_process(process_t *process, const bool children)
{
	child_t *c, *next;

	if(!process)
		return;
	if(running_process == process)
		running_process = NULL;
	if(process->parent)
	{
		c = process->parent->children;
		while(c)
		{
			next = c->next;
			if(c->process->pid == process->pid)
			{
				cache_free(children_cache, c);
				break;
			}
			c = next;
		}
	}
	c = process->children;
	while(c)
	{
		next = c->next;
		if(children)
			del_process(c->process, true);
		else
			c->process->parent = NULL;
		cache_free(children_cache, c);
		c = next;
	}
	vmem_free(process->page_dir, true);
	// TODO Free `signals_queue`
	cache_free(processes_cache, process);
}

// TODO Alloc when the process is created (because of `fork`) (or block parent?)
__attribute__((hot))
static void init_process(process_t *process)
{
	vmem_t vmem;
	void *user_stack = NULL, *kernel_stack = NULL;

	if(process->parent)
		vmem = vmem_clone(process->parent->page_dir, true);
	else
		vmem = vmem_init();
	if(!vmem)
		return;
	// TODO Change default stack size (and allow stack grow)
	// TODO Do not allow access to kernel_stack in user space?
	if(!(user_stack = vmem_alloc_pages(vmem, 1))
		|| !(kernel_stack = vmem_alloc_pages(vmem, 1)))
	{
		vmem_free(vmem, false);
		buddy_free(user_stack);
		buddy_free(kernel_stack);
		return;
	}
	process->page_dir = vmem;
	process->user_stack = user_stack;
	process->kernel_stack = kernel_stack;
	process->regs_state.esp = (uintptr_t) user_stack + (PAGE_SIZE - 1);
	process->regs_state.eip = (uintptr_t) process->begin;
	process_set_state(process, WAITING);
	printf("pid: %i user: %p kernel: %p\n", process->pid, process->user_stack, process->kernel_stack);
}

__attribute__((hot))
static process_t *next_waiting_process(process_t *process)
{
	process_t *p;

	if(!process && !(process = processes))
		return NULL;
	p = process;
	do
	{
		if(!(p = p->next))
			p = processes;
	}
	while(p != process && p->state != WAITING);
	return (p->state == WAITING ? p : NULL);
}

__attribute__((hot))
static void switch_processes(void)
{
	process_t *p;

	if(!processes)
		return;
	if(!(p = next_waiting_process(running_process)))
		return;
	process_set_state(p, RUNNING);
	tss.ss0 = GDT_KERNEL_DATA_OFFSET;
	tss.ss = GDT_USER_DATA_OFFSET;
	tss.esp0 = (uint32_t) p->kernel_stack + (PAGE_SIZE - 1);
	if(p->syscalling)
		kernel_switch(&p->regs_state,
			GDT_USER_DATA_OFFSET, GDT_USER_CODE_OFFSET);
	else
		context_switch(&p->regs_state,
			GDT_USER_DATA_OFFSET | 3, GDT_USER_CODE_OFFSET | 3, p->page_dir);
}

__attribute__((hot))
void process_tick(const regs_t *registers)
{
	process_t *p;

	vmem_kernel_restore();
	if(running_process)
		running_process->regs_state = *registers;
	p = processes;
	while(p)
	{
		switch(p->state)
		{
			case CREATED:
			{
				init_process(p);
				break;
			}

			case BLOCKED:
			{
				// TODO Unblock if needed?
				break;
			}

			default: break;
		}
		p = p->next;
	}
	switch_processes();
}
