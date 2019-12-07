#include <process/process.h>
#include <kernel.h>
#include <idt/idt.h>

void sem_init(semaphore_t *sem, const unsigned k)
{
	if(!sem)
		return;
	sem->k = k;
	sem->proc_queue = NULL;
}

void sem_wait(semaphore_t *sem, process_t *process)
{
	process_t *p;

	if(!sem || !process)
		return;
	CLI();
	printf("%i sem_wait\n", process->pid);
	if(sem->k <= 0)
	{
		printf("%i added to queue\n", process->pid);
		if((p = sem->proc_queue))
		{
			while(p->sem_next)
				p = p->sem_next;
			p->sem_next = process;
		}
		else
			sem->proc_queue = process;
		process->sem_next = NULL;
		process_set_state(process, BLOCKED);
		while(process->state != RUNNING)
			kernel_wait();
	}
	else
		--sem->k;
	printf("%i acquired sem\n", process->pid);
	STI();
}

void sem_post(semaphore_t *sem)
{
	if(!sem)
		return;
	CLI();
	printf("sem_post\n");
	++sem->k;
	if(sem->proc_queue)
	{
		process_set_state(sem->proc_queue, WAITING);
		printf("%i unblocked\n", sem->proc_queue->pid);
		sem->proc_queue = sem->proc_queue->sem_next;
	}
	STI();
}
