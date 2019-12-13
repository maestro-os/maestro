#include <process/process.h>
#include <kernel.h>
#include <idt/idt.h>

#define printf(...) // TODO rm

// TODO Add spinlock in semaphore structure?

void sem_init(semaphore_t *sem)
{
	if(!sem)
		return;
	bzero(sem, sizeof(semaphore_t));
}

static void sem_add(semaphore_t *sem, process_t *process)
{
	process_t *p;

	if((p = sem->proc_queue))
	{
		while(p->sem_next)
			p = p->sem_next;
		p->sem_next = process;
	}
	else
		sem->proc_queue = process;
	process->sem_curr = sem;
	process->sem_next = NULL;
	process_set_state(process, BLOCKED);
}

void sem_wait(semaphore_t *sem, process_t *process)
{

	if(!sem || !process || sem->proc_current == process)
		return;
	CLI();
	printf("\n%i sem_wait\n", process->pid);
	if(sem->proc_current)
	{
		printf("\n%i added to queue\n", process->pid);
		sem_add(sem, process);
		while(sem->proc_current != process)
		{
			printf("\n%i sem_waiting\n", process->pid);
			asm("sti; hlt; cli");
		}
	}
	else
		sem->proc_current = process;
	printf("\n%i acquired sem\n", process->pid);
	STI();
}

void sem_remove(semaphore_t *sem, process_t *process)
{
	process_t *p;

	if(!sem || !process)
		return;
	printf("\n%i sem_remove\n", process->pid);
	if(process == sem->proc_current)
	{
		sem->proc_current = sem->proc_queue;
		if(sem->proc_queue)
			sem->proc_queue = sem->proc_queue->sem_next;
		process_set_state(sem->proc_current, WAITING);
		if(sem->proc_current) // TODO rm
		{
			printf("\n%i unblocked\n", sem->proc_current->pid);
		}
	}
	else
	{
		p = sem->proc_queue;
		while(p)
		{
			if(p->sem_next == process)
			{
				p->sem_next = p->sem_next->sem_next;
				break;
			}
			p = p->sem_next;
		}
	}
	process->sem_curr = NULL;
	process->sem_next = NULL;
}

void sem_post(semaphore_t *sem)
{
	if(!sem)
		return;
	CLI();
	printf("\nsem_post\n");
	sem_remove(sem, sem->proc_current);
	STI();
}
