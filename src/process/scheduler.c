#include <process/scheduler.h>
#include <process/process.h>
#include <process/process_internal.h>
#include <memory/vmem/vmem.h>
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
 * The scheduler works in a round robin fashion. There is a linked list of
 * WAITING processes with a cursor on it. When selecting the next process, the
 * scheduler just moves the cursor to the next element in the list.
 *
 * When a process's state is changed, if new state is:
 * - WAITING: the process is inserted into the scheduler's list
 * - non-WAITING: the process is removed from the scheduler's list
 *
 * Thus, the list only contains WAITING processes.
 *
 * This algorithm has complexity O(1) because it never has to iterate over the
 * list.
 *
 * The time slice during which a process is executed is called a `quantum`.
 * The number of quantum for a process is defined by its priority.
 */

// TODO Spinlock?
// TODO Use `CYCLE_LENGTH`

/*
 * The cursor of the scheduler.
 */
static list_head_t *cursor = NULL;

/*
 * The list of processes for scheduling.
 */
static list_head_t *scheduler_processes = NULL;

/*
 * The number of elapsed quantum for the current process.
 */
static unsigned curr_quantum = 0;

/*
 * Returns the currently running process. Returns NULL if no process is running.
 */
ATTR_HOT
process_t *get_running_process(void)
{
	return running_process;
}

/*
 * Adds the process to the scheduler queue.
 */
void scheduler_add(process_t *p)
{
	debug_assert(sanity_check(p), "scheduler: invalid argument");
	list_insert_front(&scheduler_processes, &p->schedule_list);
	if(!cursor)
		cursor = scheduler_processes;
}

/*
 * Removes the process from the scheduler queue.
 */
void scheduler_remove(process_t *p)
{
	debug_assert(sanity_check(p), "scheduler: invalid argument");
	if(cursor == &p->schedule_list)
		cursor = scheduler_processes;
	list_remove(&scheduler_processes, &p->schedule_list);
}

/*
 * Returns the number of quantum for the given process.
 */
static unsigned get_quantum_count(const process_t *process)
{
	unsigned quantum;

	debug_assert(sanity_check(process), "scheduler: invalid argument");
	quantum = 128 + process->priority;
	// TODO Must be bound into CYCLE_LENGTH
	if(quantum <= 0)
		quantum = 1;
	return quantum;
}

/*
 * Returns the next waiting process to be run.
 */
ATTR_HOT
static process_t *next_waiting_process(void)
{
	list_head_t *l;
	process_t *p;

	if(!running_process || ++curr_quantum >= get_quantum_count(running_process))
	{
		if(!cursor)
			cursor = scheduler_processes;
		if((l = cursor))
		{
			p = CONTAINER_OF(l, process_t, schedule_list);
			debug_assert(p->state == WAITING,
				"scheduler: invalid state for process");
			curr_quantum = 0;
		}
		else
			p = NULL;
	}
	else
		p = running_process;
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

	if(!(p = next_waiting_process()))
		return;
	if(running_process && p != running_process)
		process_set_state(running_process, WAITING);
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
