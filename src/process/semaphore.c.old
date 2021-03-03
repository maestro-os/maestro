#include <process/process.h>
#include <kernel.h>
#include <idt/idt.h>

/*
 * Initializes the given semaphore. This function must not be called if the
 * semaphore is already initialized.
 */
void sem_init(semaphore_t *sem)
{
	debug_assert(sanity_check(sem), "semaphore: invalid argument");
	bzero(sem, sizeof(semaphore_t));
}

/*
 * Adds the given process `process` to the given semaphore `sem`.
 */
static void sem_add(semaphore_t *sem, process_t *process)
{
	debug_assert(sanity_check(sem) && sanity_check(process),
		"semaphore: invalid arguments");
	list_insert_front(&sem->proc_queue_last, &process->sem_queue);
	process->sem_curr = sem;
	process_set_state(process, BLOCKED);
}

/*
 * Makes the given `process` aquire the resource of the semaphore. If not
 * available, the function adds the process to the queue, changing its state to
 * BLOCKED and makes the current context wait until the resource gets available
 * again for the process.
 */
void sem_wait(semaphore_t *sem, process_t *process)
{
	debug_assert(sanity_check(sem) && sanity_check(process),
		"semaphore: invalid arguments");
	CLI();
	spin_lock(&sem->spinlock);
	if(sem->proc_current == process)
		goto end;
	if(sem->proc_current)
	{
		sem_add(sem, process);
		spin_unlock(&sem->spinlock);
		while(sem->proc_current != process)
			asm("sti; hlt; cli");
	}
	else
	{
		sem->proc_current = process;
		spin_unlock(&sem->spinlock);
	}

end:
	STI();
}

/*
 * Removes the process `process` from the queue of the semaphore `sem`.
 */
void sem_remove(semaphore_t *sem, process_t *process)
{
	list_head_t *n;

	debug_assert(sanity_check(sem) && sanity_check(process),
		"semaphore: invalid arguments");
	CLI();
	spin_lock(&sem->spinlock);
	if(process == sem->proc_current)
	{
		sem->proc_current = CONTAINER_OF(sem->proc_queue, process_t, sem_queue);
		n = sem->proc_queue;
		if(sem->proc_queue == n)
			sem->proc_queue = sem->proc_queue->prev;
		list_remove(&sem->proc_queue_last, n);
	}
	else
	{
		if(sem->proc_queue == &process->sem_queue)
			sem->proc_queue = sem->proc_queue->prev;
		list_remove(&sem->proc_queue_last, &process->sem_queue);
	}
	process->sem_curr = NULL;
	// TODO Potential problem if process is TERMINATED? (falling back to prev state)
	process_set_state(sem->proc_current, sem->proc_current->prev_state);
	spin_unlock(&sem->spinlock);
	STI();
}

/*
 * Makes the current process release the semaphore `sem`, making it available
 * for the next process in the queue.
 */
void sem_post(semaphore_t *sem)
{
	debug_assert(sanity_check(sem), "semaphore: invalid argument");
	debug_assert(sanity_check(sem->proc_current),
		"semaphore: no process is using the semaphore");
	sem_remove(sem, sem->proc_current);
}
