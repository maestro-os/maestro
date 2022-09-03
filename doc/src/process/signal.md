# Signal

TODO



## SIGSYS

When a process tries to use a system call whose ID doesn't correspond to any known system call, the process shall be killed by the kernel with a SIGSYS signal.
Such a signal cannot be caught and results in the termination of the process.
