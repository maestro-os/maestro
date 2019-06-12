#include "process.h"
#include "../libc/errno.h"

static cache_t *processes_cache;
static process_t *processes, *current_process;

static tss_entry_t tss_entry;

__attribute__((cold))
static void tss_init(void)
{
	const uint32_t base = (uint32_t) &tss_entry;
	const unsigned limit = sizeof(tss_entry_t) - 1;
	const uint8_t flags = 0x4;
	const uint8_t access = 0x89;

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
		NULL, bzero);
	if(!processes_cache) PANIC("Cannot allocate cache for processes!");

	processes = NULL;
	current_process = NULL;

	tss_init();
	// TODO CPU time divison (timing, philosophers, etc...)
}

__attribute__((hot))
static process_t *alloc_process(const pid_t pid, const pid_t parent)
{
	errno = 0;

	if(pid <= 0)
	{
		errno = EINVAL;
		return NULL;
	}

	process_t *process;
	if(!(process = cache_alloc(processes_cache)))
	{
		errno = ENOMEM;
		return NULL;
	}

	if(parent > 0 && !(process->page_dir = buddy_alloc(0)))
	{
		errno = ENOMEM;
		cache_free(processes_cache, process);
		return NULL;
	}

	process->pid = pid;
	process->parent = parent;

	return process;
}

__attribute__((hot))
static process_t *create_process(const pid_t parent)
{
	errno = 0;
	process_t *p;

	if(parent > 0)
	{
		if(!(p = get_process(parent)))
		{
			errno = ESRCH;
			return NULL;
		}

		while(p->next && p->next->pid - p->pid > 1)
			p = p->next;
	}
	else
		p = processes;

	const pid_t pid = (p ? p->pid + 1 : 1);
	process_t *new_proc;
	if(!(new_proc = alloc_process(pid, parent))) return NULL;

	if(p)
	{
		process_t *tmp = p->next;

		p->next = new_proc;
		tmp->prev = new_proc;
		new_proc->next = tmp;
		new_proc->prev = p;
	}
	else
		processes = new_proc;

	return new_proc;
}

// TODO Spinlock
__attribute__((hot))
pid_t kfork(const pid_t parent)
{
	errno = 0;

	if(parent < 0)
	{
		errno = EINVAL;
		return -1;
	}

	process_t *process;
	if(!(process = create_process(parent)))
		return -1;
	return process->pid;
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
static void suspend_process(process_t *process)
{
	if(!process) return;

	// TODO
}

__attribute__((hot))
static void resume_process(process_t *process)
{
	if(!process) return;

	// TODO
	//if(process->page_dir)
		//paging_enable(process->page_dir);
}

void process_tick(void)
{
	// TODO Multicore handling

	if(current_process)
	{
		suspend_process(current_process);

		if(current_process->next)
			current_process = current_process->next;
		else
			current_process = processes;
	}
	else
		current_process = processes;

	resume_process(current_process);
}
