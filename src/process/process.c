#include "process.h"
#include "../memory/memory.h"
#include "../util/linked_list.h"

#define LIST_PROCESS(list)	((process_t *) list->content)
#define LIST_PID(list)		(LIST_PROCESS(list)->pid)

static list_t *processes;

void process_init()
{
	processes = NULL;
}

static process_t *insert_process(list_t *node, const pid_t pid)
{
	if(!node || pid <= 0) return NULL;

	process_t *process;
	if(!(process = kmalloc(sizeof(process_t)))) return NULL;
	bzero(process, sizeof(process));

	if(!(process->page_dir = kmalloc(PAGE_SIZE))) // TODO Must be aligned
	{
		kfree(process);
		return NULL;
	}

	list_t *l;
	if(!(l = kmalloc(sizeof(list_t))))
	{
		kfree(process->page_dir);
		kfree(process);
		return NULL;
	}
	bzero(l, sizeof(list_t));
	l->content = process;

	list_t *tmp = node->next;
	node->next = l;
	l->next = tmp;

	return process;
}

static process_t *create_process(const pid_t parent)
{
	if(parent < 0) return NULL;
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
			return insert_process(l, pid + 1);

		l = l->next;
	}

	return NULL;
}

extern void switch_user_mode();

// TODO Make atomic
pid_t kfork(const pid_t parent)
{
	process_t *process;
	if(!(process = create_process(parent))) return -1;

	paging_enable(process->page_dir);
	// TODO Switch to user mode
	return process->pid;
}

process_t *get_process(const pid_t pid)
{
	if(pid == 0) return NULL;
	// TODO

	return NULL;
}
