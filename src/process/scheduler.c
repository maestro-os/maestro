#include <process/scheduler.h>
#include <process/process.h>
#include <process/process_internal.h>
#include <memory/memory.h>
#include <kernel.h>

/*
 * The scheduler is the part of the kernel which decides of CPU time repartition
 * between processes.
 *
 * It also handles context switching. Special considerations must be taken
 * toward privilege levels. If a process is interrupted while executing a
 * system call, it should be resumed at the same level of permission.
 *
 * In x86, to prevent some vulnerabilities such as Meltdown for example, the
 * page directory should be switched to the kernel's one. However, the stack has
 * to stay at the same position, thus the kernel stack must be identity mapped.
 *
 * The scheduler uses queues to sort processes by priority. High priority
 * processes have more CPU time.
 */

// TODO Spinlock?

/*
 * The list of queues sorted by growing priority.
 */
static schedule_queue_t queues[QUEUES_COUNT];

/*
 * The time unit, used to divide CPU time.
 */
static unsigned time_unit = 0;

/*
 * Returns the currently running process. Returns NULL if no process is running.
 */
ATTR_HOT
process_t *get_running_process(void)
{
	return running_process;
}

/*
 * Returns the queue for the given `priority`.
 */
static inline size_t get_queue_id(int8_t priority)
{
	int id;
	
	id = ((int) 128 + priority) * (QUEUES_COUNT - 1) / 255;
	debug_assert(id > 0 && id < QUEUES_COUNT, "scheduler: invalid queue id");
	return id;
}

/*
 * Adds the process to the scheduler queue.
 */
void scheduler_add(process_t *p)
{
	schedule_queue_t *queue;

	debug_assert(sanity_check(p), "scheduler: invalid argument");
	queue = &queues[get_queue_id(p->priority)];
	list_insert_front(&queue->list, &p->schedule_queue);
	if(!queue->cursor)
		queue->cursor = queue->list;
}

/*
 * Removes the process from the scheduler queue.
 */
void scheduler_remove(process_t *p)
{
	schedule_queue_t *queue;

	debug_assert(sanity_check(p), "scheduler: invalid argument");
	queue = &queues[get_queue_id(p->priority)];
	if(queue->cursor == &p->schedule_queue)
		queue->cursor = queue->list;
	list_remove(&queue->list, &p->schedule_queue);
}

/*
 * Returns the next waiting process to be run.
 */
ATTR_HOT
static process_t *next_waiting_process(void)
{
	unsigned id;
	schedule_queue_t *queue;
	process_t *p;

	id = (SQRT(8 * time_unit + 1) - 1) / 2;
	debug_assert(id < QUEUES_COUNT, "scheduler: invalid queue id");
	queue = &queues[id];
	if(!queue->cursor && !(queue->cursor = queue->list))
		return NULL;
	p = CONTAINER_OF(queue->cursor, process_t, schedule_queue);
	if(!(queue->cursor = queue->cursor->next))
		queue->cursor = queue->list;
	time_unit = (time_unit + 1) % CYCLE_LENGTH;
	return p;
}

/*
 * Switches context to the given process `process`.
 */
ATTR_HOT
static void switch_process(process_t *process)
{
	int syscalling;

	debug_assert(sanity_check(process), "process: invalid argument");
	process_set_state(process, RUNNING);
	tss.ss0 = GDT_KERNEL_DATA_OFFSET;
	tss.ss = GDT_USER_DATA_OFFSET;
	tss.esp0 = (uint32_t) process->kernel_stack;
	syscalling = process->syscalling;
	debug_assert(sanity_check(process->mem_space->page_dir),
		"process: bad memory context");
	paging_enable(process->mem_space->page_dir);
	if(syscalling)
		kernel_switch(&process->regs_state);
	else
		context_switch(&process->regs_state,
			GDT_USER_DATA_OFFSET | 3, GDT_USER_CODE_OFFSET | 3);
}

/*
 * Switches to the next process to be run.
 */
ATTR_HOT
static void switch_processes(void)
{
	process_t *p;

	// TODO Rewrite processes scheduling
	if(!(p = next_waiting_process()) && !(p = running_process))
		return;
	switch_process(p);
}

/*
 * The ticking function, invoking the processes scheduler.
 */
ATTR_HOT
ATTR_NORETURN
void scheduler_tick(const regs_t *registers)
{
	spin_lock(&processes_spinlock); // TODO Spinlock on `running_process`?
	if(running_process)
		running_process->regs_state = *registers;
	spin_unlock(&processes_spinlock);
	switch_processes();
	kernel_halt();
}
