# Process

A process is a program being executed by the kernel. Each process has a unique PID which is allocated at creation.



## State

A process can have the following states:

| Name       | Associated character | Description                                                                              |
|------------|----------------------|------------------------------------------------------------------------------------------|
| `Running`  | R                    | The process is currently running or is ready to be resumed by the scheduler              |
| `Sleeping` | S                    | The process is waiting on a resource to become available (usualy I/O or another process) |
| `Stopped`  | T                    | The process is paused                                                                    |
| `Zombie`   | Z                    | The process has been terminated and cannot resume, ever                                  |

The `Running` state is the only state in which a process can be executed.

The following transitions are valid:
- R -> S
- (R|S) -> T
- (S|T) -> R
- (R|S|T) -> Z



## Scheduler

The scheduler is a component that decide which process is running, and when.

The system triggers interruptions periodically in order to interrupt the current process, then the scheduler determines the next process to be run, and switches context to that process.

The frequency of interruption is determined by the number of processes in running state.

To determine the next process to be run, the scheduler uses different informations such as state and priority of the process.
