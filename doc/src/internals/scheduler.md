# Scheduler

> For more details about processes, check [this page](../process.md).

The scheduler preempts execution of the currently running process to switch context to another process in its run queue, in order to share CPU-time across processes. Each CPU core has its own scheduler.

When a process transitions into `Running` state, it is inserted into the run queue of a scheduler. The kernel attempts to balance processes across CPU cores.

Likewise, when a process transitions into another state than `Running`, it is **dequeued** (removed from its run queue).

Context switching can be triggered by a timer interrupt or when waiting for a resource to become available (for example).
It can also be triggered manually by calling `schedule`.

If there is no process in a scheduler's run queue, it shall switch to the **idle task**, which is a kernel thread with PID `0` that puts the CPU in idle state.

## Critical sections

Sometimes, we want to be able to process interrupts, but prevent the scheduler from preempting the process.

In order to achieve this, we have to use **critical sections**.
A critical section is entered by calling `preempt_disable`, and is exited by calling `preempt_enable`. To ensure correctness, one should prefer using the `critical` function.

Note that:
- `preempt_enable` or `critical` may preempt the execution context before returning
- calling `schedule` inside a critical section is invalid (for obvious reasons)

Critical sections can be nested. This is handled with a per-CPU counter.
