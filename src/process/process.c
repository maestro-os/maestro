#include <process/process.h>
#include <libc/errno.h>

// TODO
#include <debug/debug.h>
#include <tty/tty.h>

// TODO Set errnos
// TODO Multicore handling

static cache_t *processes_cache;
static cache_t *children_cache;
static cache_t *signals_cache;

static process_t *volatile processes = NULL;
static process_t *volatile running_process = NULL;
static uint8_t *pids_bitmap;

__ATTR_PAGE_ALIGNED
__ATTR_BSS
static tss_entry_t tss;

static spinlock_t spinlock = 0;

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

__attribute__((hot))
process_t *new_process(process_t *parent, const regs_t *registers)
{
	pid_t pid;
	process_t *new_proc, *p;

	spin_lock(&spinlock);
	errno = 0;
	if((pid = alloc_pid()) < 0
		|| !(new_proc = cache_alloc(processes_cache)))
	{
		free_pid(pid);
		errno = ENOMEM;
		spin_unlock(&spinlock);
		return NULL;
	}
	new_proc->pid = pid;
	new_proc->parent = parent;
	new_proc->regs_state = *registers;
	process_add_child(parent, new_proc);
	if(errno)
	{
		// TODO Free all
		spin_unlock(&spinlock);
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
	spin_unlock(&spinlock);
	return new_proc;
}

__attribute__((hot))
process_t *get_process(const pid_t pid)
{
	process_t *p;

	spin_lock(&spinlock);
	errno = 0;
	p = processes;
	if(pid <= 0)
	{
		errno = EINVAL;
		spin_unlock(&spinlock);
		return NULL;
	}
	while(p)
	{
		if(p->pid == pid)
		{
			spin_unlock(&spinlock);
			return p;
		}
		p = p->next;
	}
	errno = ESRCH;
	spin_unlock(&spinlock);
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
	if(!(p = new_process(proc, &proc->regs_state)))
		return NULL;
	if(!(p->page_dir = vmem_clone(proc->page_dir, 1)))
	{
		del_process(p, 0);
		return NULL;
	}
	p->state = WAITING;
	return p;
}

__attribute__((hot))
void process_set_state(process_t *process, const process_state_t state)
{
	if(!process)
		return;
	spin_lock(&spinlock);
	if(state == RUNNING)
	{
		if(running_process)
		{
			running_process->prev_state = running_process->state;
			running_process->state = WAITING;
			running_process = NULL;
		}
		running_process = process;
	}
	else if(process == running_process)
		running_process = NULL;
	process->prev_state = process->state;
	process->state = state;
	spin_unlock(&spinlock);
}

__attribute__((hot))
void process_add_child(process_t *parent, process_t *child)
{
	child_t *c;

	if(!parent || !child)
		return;
	spin_lock(&parent->spinlock);
	if(!(c = cache_alloc(children_cache)))
	{
		errno = ENOMEM;
		spin_unlock(&parent->spinlock);
		return;
	}
	c->next = parent->children;
	c->process = child;
	parent->children = c;
	spin_unlock(&parent->spinlock);
}

__attribute__((hot))
void process_exit(process_t *proc, const int status)
{
	if(!proc)
		return;
	spin_lock(&proc->spinlock);
	proc->status = status;
	process_set_state(proc, TERMINATED);
	if(running_process == proc)
		running_process = NULL;
	spin_unlock(&proc->spinlock);
}

// TODO Limit on signals?
// TODO Perform signals directly?
// TODO Execute signal later?
__attribute__((hot))
void process_kill(process_t *proc, const int sig)
{
	signal_t *s;
	sigaction_t *action;

	if(!proc)
		return;
	spin_lock(&proc->spinlock);
	if(sig == SIGKILL || sig == SIGSTOP
		|| (action = proc->sigactions + sig)->sa_handler == SIG_DFL)
	{
		signal_default(proc, sig);
		spin_unlock(&proc->spinlock);
		return;
	}
	if(action->sa_handler == SIG_IGN || !(s = cache_alloc(signals_cache)))
	{
		spin_unlock(&proc->spinlock);
		return;
	}
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
	spin_unlock(&proc->spinlock);
}

__attribute__((hot))
void del_process(process_t *process, const int children)
{
	child_t *c, *next;

	if(!process)
		return;
	spin_lock(&spinlock);
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
		if(children) // TODO Usefull?
			del_process(c->process, 1);
		else
			c->process->parent = NULL;
		cache_free(children_cache, c);
		c = next;
	}
	vmem_free(process->page_dir, 1);
	// TODO Free `signals_queue`
	cache_free(processes_cache, process);
	spin_unlock(&spinlock);
}

// TODO Alloc when the process is created (because of `fork`) (or block parent?)
__attribute__((hot))
static void init_process(process_t *process)
{
	vmem_t vmem;
	void *user_stack = NULL, *kernel_stack = NULL;

	if(!process->page_dir)
	{
		// TODO Change default stack size (and allow stack grow)
		// TODO Do not allow access to kernel_stack in user space?
		if(!(vmem = vmem_init()) || !(user_stack = vmem_alloc_pages(vmem, 1))
			|| !(kernel_stack = vmem_alloc_pages(vmem, 1)))
		{
			vmem_free(vmem, 0);
			buddy_free(user_stack);
			buddy_free(kernel_stack);
			return;
		}
		process->page_dir = vmem;
		process->user_stack = user_stack;
		process->kernel_stack = kernel_stack;
		process->regs_state.esp = (uintptr_t) user_stack + (PAGE_SIZE - 1);
	}
	process_set_state(process, WAITING);
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

	if(!processes || !(p = next_waiting_process(running_process)))
		return;
	printf("%i tries to eat\n", p->pid);
	process_set_state(p, RUNNING);
	tss.ss0 = GDT_KERNEL_DATA_OFFSET;
	tss.ss = GDT_USER_DATA_OFFSET;
	tss.esp0 = (uint32_t) p->kernel_stack + (PAGE_SIZE - 1);
	paging_enable(p->page_dir);
	if(p->syscalling)
		kernel_switch(&p->regs_state);
	else
		context_switch(&p->regs_state,
			GDT_USER_DATA_OFFSET | 3, GDT_USER_CODE_OFFSET | 3);
}

__attribute__((hot))
void process_tick(const regs_t *registers)
{
	process_t *p;

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
	// TODO Uncomment
	/*if(!processes)
		kernel_halt();*/
}
