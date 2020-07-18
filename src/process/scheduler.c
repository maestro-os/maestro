#include <process/scheduler.h>
#include <process/process.h>
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
 */

extern avl_tree_t *processes;

extern process_t *running_process;

extern tss_entry_t tss;

extern spinlock_t processes_spinlock;

/*
 * Returns the currently running process. Returns NULL if no process is running.
 */
ATTR_HOT
process_t *get_running_process(void)
{
	return running_process;
}

/*
 * Returns the next waiting process to be run.
 */
ATTR_HOT
static process_t *next_waiting_process(void)
{
	// TODO
	if(!processes)
		return NULL;
	return CONTAINER_OF(processes, process_t, tree);
/*	process_t *p;
	int loop = 0;

	spin_lock(&processes_spinlock);
	if(!(p = running_process))
		p = processes;
	if(!p)
		goto end;
loop:
	if(!(p = p->next))
	{
		if(loop)
		{
			p = NULL;
			goto end;
		}
		p = processes;
		loop = 1;
	}
	if(!p)
		goto end;
	spin_lock(&p->spinlock);
	if(p->state != WAITING)
	{
		spin_unlock(&p->spinlock);
		goto loop;
	}
	else
		spin_unlock(&p->spinlock);

end:
	spin_unlock(&processes_spinlock);
	return p;*/
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
	spin_lock(&processes_spinlock);
	// TODO Spinlock on `running_process`?
	if(running_process)
		running_process->regs_state = *registers;
	spin_unlock(&processes_spinlock);
	// TODO
	/*profiler_capture();
	profiler_print();*/
	switch_processes();
	kernel_halt();
}
