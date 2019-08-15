# Processes

Processes are tasks running on the computer. They have several attributes:
- **PID**: Process ID, an unique identifier associated to each process
- **state**: The current state of the process, which is one of the following:
	- **CREATED**: The process has been created and is waiting for resources in order to be runned
	- **WAITING**: The process is waiting to resume
	- **RUNNING**: The process is currently running
	- **BLOCKED**: The process is waiting for data
	- **TERMINATED**: The process has terminated and is waiting for its parent process to retrieve its status
- **owner_id**: The identifier of the user who owns the process
- **parent**: The parent process
- **children**: Children processes
- **page_dir**: The virtual memory space descriptor associated with the process
- **user_stack**: The top of the process's user side stack
- **kernel_stack**: The top of the process's kernel side stack
- **tss**: The state of registers for context switch
- **signals_queue**: The queue of waiting signals for the process
- **exit_status**: The exit status of the process
- **begin**: The pointer to the first instruction of the process
TODO: Signals tss and stack

The TSS is a special segment used to perform task switching.
On context switch, the content of registers are swapped with the content of the TSS, allowing to replace all at once.

Processes are running on Ring 3, thus the less privileged level of permissions. Every process has its own memory space which can be extended by allocating memory using a specific system call.
Processes have one page (4096 bytes) of memory available in the beginning of the process.
If the stack grows too much and crosses that limit, the kernel allows more pages in the end of the stack to allow the process to go further. The default limit is 2048 pages (8 MB).
If the stack limit is crossed, no further pages are allocated and a SIGSEGV signal is sent to the process.
