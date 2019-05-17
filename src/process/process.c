#include "process.h"
#include "../memory/memory.h"
#include "../libc/errno.h"
#include "../util/linked_list.h"

static cache_t *processes_cache;
static list_t *processes;

__attribute__((cold))
void process_init()
{
	processes_cache = cache_create("processes", sizeof(process_t), PID_MAX,
		NULL, bzero);
	if(!processes_cache) PANIC("Cannot allocate cache for processes!");

	processes = NULL;
}

__attribute__((hot))
static process_t *insert_process(list_t **node, const pid_t pid)
{
	if(!node || pid <= 0)
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

	if(!(process->page_dir = buddy_alloc(0)))
	{
		errno = ENOMEM;
		cache_free(processes_cache, process);
		return NULL;
	}

	list_t *l;
	if(!(l = kmalloc(sizeof(list_t))))
	{
		errno = ENOMEM;
		kfree(process->page_dir);
		kfree(process);
		return NULL;
	}

	l->content = process;

	if(*node)
	{
		list_t *tmp = *node;
		*node = l;
		l->next = tmp;
	}
	else
		*node = l;

	return process;
}

__attribute__((hot))
static process_t *create_process(const pid_t parent)
{
	if(parent < 0)
	{
		errno = EINVAL;
		return NULL;
	}

	list_t *l = processes;

	if(parent > 0)
	{
		while(l->next)
		{
			if(LIST_PID(l) == parent) break;
			l = l->next;
		}
	}

	while(l)
	{
		const pid_t pid = LIST_PID(l);
		if(!l->next || LIST_PID(l->next) > pid + 1)
			return insert_process(&l, pid + 1);

		l = l->next;
	}

	// TODO Potential pid collision?
	return insert_process(&l, parent + 1);
}

// TODO Spinlock
__attribute__((hot))
pid_t kfork(const pid_t parent)
{
	errno = 0;

	process_t *process;
	if(!(process = create_process(parent)))
		return -1;

	// TODO Fill page directory and enable now?
	// paging_enable(process->page_dir);
	// TODO Switch to user mode now?
	// switch_usermode();

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

	// TODO

	return NULL;
}
