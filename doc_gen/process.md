```
typedef struct process
```

Structure describing a process.

- **next**: The next process (processes are stored into a linked list)
- **pid**: The process's id
- **state**: The current process state
	- **CREATED**: The process has been created and is waiting for resouces to be available
	- **WAITING**: The process is paused, waiting to be resumed by the kernel
	- **RUNNING**: The process is running
	- **BLOCKED**: The process is waiting for another process
	- **STOPPED**: The process is paused and will not resume until it receives a signal to unstop it
	- **TERMINATED**: The process has been terminated and is waiting for its parent process to get its exit status
- **prev_state**: The previous process state
- **owner_id**: Id of the user owning the process
- **parent**: A pointer to the parent process (**NULL** if orphan)
- **children**: The list of children process
- **page_dir**: The page directory for the process
- **user_stack**: Pointer to the top of the user space stack
- **kernel_stack**: Pointer to the top of the kernel space stack
- **regs_state**: Saved state of the process's registers
- **syscalling**: Tells whether the process is currently making a system call
- **sigactions**: The actions to be performed for each signal
- **signals_queue**: The first signal of the signals queue
- **last_signal**: The last signal of the signals queue
- **status**: The exit status of the process (valid only if `state == TERMINATED`)
- **spinlock**: The structure's spinlock
