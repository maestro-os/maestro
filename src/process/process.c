#include <process/process.h>
#include <libc/errno.h>

static cache_t *processes_cache;
static cache_t *children_cache;
static process_t *processes;

static uint8_t *pids_bitmap;

static tss_entry_t tss_entry;

__attribute__((hot))
static void process_ctor(void *ptr, size_t size)
{
	bzero(ptr, size);
	if(CREATED != 0)
		((process_t *) ptr)->state = CREATED;
}

__attribute__((cold))
static void tss_init(void)
{
	const uint32_t base = (uint32_t) &tss_entry;
	const unsigned limit = sizeof(tss_entry_t);
	const uint8_t flags = 0b0100;
	const uint8_t access = 0b10001001;

	gdt_entry_t *tss_gdt = tss_gdt_entry();
	bzero(tss_gdt, sizeof(gdt_entry_t));
	tss_gdt->limit_low = limit & 0xffff;
	tss_gdt->base_low = base & 0xffff;
	tss_gdt->base_mid = (base >> 16) & 0xff;
	tss_gdt->access = access;
	tss_gdt->flags_limit = ((limit >> 16) & 0xf) | (flags << 4);
	tss_gdt->base_high = (base >> 24) & 0xff;

	bzero(&tss_entry, sizeof(tss_entry_t));
	tss_flush();
}

__attribute__((cold))
void process_init(void)
{
	processes_cache = cache_create("processes", sizeof(process_t), PID_MAX,
		process_ctor, bzero);
	children_cache = cache_create("process_children", sizeof(child_t), PID_MAX,
		NULL, bzero);

	if(!processes_cache || !children_cache)
		PANIC("Cannot allocate caches for processes!", 0);

	processes = NULL;

	if(!(pids_bitmap = kmalloc_zero(PIDS_BITMAP_SIZE)))
		PANIC("Cannot allocate PIDs bitmap!", 0);
	bitmap_set(pids_bitmap, 0);

	tss_init();
}

__attribute__((hot))
static pid_t alloc_pid(void)
{
	// TODO Use a last_pid variable to avoid searching from the first pid
	const pid_t pid = bitmap_first_clear(pids_bitmap, PIDS_BITMAP_SIZE);
	if(pid >= (pid_t) PIDS_BITMAP_SIZE) return -1;

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
	errno = 0;

	pid_t pid;
	process_t *new_proc;

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
	new_proc->tss.eip = (uintptr_t) begin;

	// TODO Set child in parent (alloc child)

	if(processes)
	{
		process_t *p = processes;
		while(p->next && p->next->pid < pid)
			p = p->next;

		new_proc->next = p->next;
		p->next = new_proc;
	}
	else
		processes = new_proc;

	return new_proc;
}

process_t *get_process(const pid_t pid)
{
	errno = 0;

	if(pid <= 0)
	{
		errno = EINVAL;
		return NULL;
	}

	process_t *p = processes;

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
void del_process(process_t *process, const bool children)
{
	if(!process) return;

	child_t *c, *next;

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
		cache_free(children_cache, c);
		c = next;
	}

	vmem_free(process->page_dir, true);
	// TODO Free `signals_queue`
	cache_free(processes_cache, process);
}

__attribute__((hot))
static void init_process(process_t *process)
{
	vmem_t vmem;

	if(process->parent)
		vmem = vmem_clone(process->parent->page_dir, true);
	else
		vmem = vmem_init();
	if(!vmem)
		return;

	void *user_stack = NULL, *kernel_stack = NULL;
	// TODO Change default stack size (and allow stack grow)
	if(!(user_stack = vmem_alloc_pages(vmem, 0))
		|| !(kernel_stack = vmem_alloc_pages(vmem, 0)))
	{
		vmem_free(vmem, false);
		buddy_free(user_stack);
		buddy_free(kernel_stack);
		return;
	}

	process->page_dir = vmem;
	process->user_stack = user_stack;
	process->kernel_stack = kernel_stack;
	process->tss.cr3 = (uintptr_t) vmem;
	process->tss.esp = (uintptr_t) user_stack + PAGE_SIZE - 1; // TODO
	process->state = WAITING;
}

__attribute__((hot))
static process_t *first_running_process(void)
{
	process_t *p = processes;
	while(p && p->state != RUNNING)
		p = p->next;

	return p;
}

__attribute__((hot))
static process_t *next_waiting_process(process_t *process)
{
	process_t *p = process;

	do
	{
		if(!(p = p->next))
			p = processes;
	}
	while(p != process && p->state != RUNNING);

	return (p == process ? NULL : p);
}

__attribute__((hot))
static void switch_processes(void)
{
	if(!processes)
		return;

	process_t *p;
	if(!(p = first_running_process()))
	{
		if(processes->state == WAITING)
			p = processes;
		else
			p = next_waiting_process(processes);
	}
	else
	{
		p->state = WAITING;
		p = next_waiting_process(p);
	}

	if(!p)
		return;

	// TODO Enable paging on kernel?
	p->state = RUNNING;
	tss_entry = p->tss;
	context_switch((void *) tss_entry.esp0, (void *) tss_entry.eip);
}

void process_tick(void)
{
	// TODO Multicore handling
	switch_processes();

	process_t *p = processes;

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
}
