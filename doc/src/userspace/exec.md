# Userspace

The userspace is the place where user programs run.



## Protection rings (x86)

On x86, there is a concept of protection ring, which allow to specify the permissions of the code that is currently running.

For more details about how protection rings work, read [Wikipedia](https://en.wikipedia.org/wiki/Protection_ring).

The userspace runs on **ring 3**, whereas kernel code runs on **ring 0**. Rings 1 and 2 are not in use.

When an interruption occurs, the CPU switches back to the **ring 0** to handle it. This is the case for example when executing a system call, allowing processes to communicate with the kernel.

Context switching is the action of changing the current protection ring. The following cases can occur:
- **ring 3 -> ring 0**: registers have to be saved to prevent the kernel from overwriting them, which would corrupt the execution flow of the process
- **ring 0 -> ring 3**: registers state that have previously been saved is restored to resume the process's execution. When executing a syscall, a register may have been altered to return a result
- **ring 0 -> ring 0** can occur in the following cases:
    - an interrupt has to be handled while handling an interrupt with a lower priority or a system call: in this case, registers are saved
    - the scheduler resumes the execution of a process that has been interrupted while executing a system call: in this case, registers are restored from the previous saved state



## System call

System calls are the main way for programs to communicate with the kernel.

The system call table can be found at the root of the module `kernel::syscall`.



### Sycall ABI by architecture

| Architecture  | Description |
|---------------|-------------|
| x86 (32 bits) | This userspace first places the syscall ID in the `eax` register |
|               | Then, arguments are placed in registers in the following order: |
|               | - `ebx` |
|               | - `ecx` |
|               | - `edx` |
|               | - `esi` |
|               | - `edi` |
|               | - `ebp` |
|               | Executing instruction `int $0x80` triggers the system call |
|               | Then, the result can be retrieved from the `eax` register. Errnos are represented by negative values greater than `-4096` |



## Memory protection

Kernel code can reach user code's memory, but user code cannot reach kernel code's memory for obvious reasons.

However, some system calls can pass memory pointers to the kernel, in which case, the kernel has to make sure the userspace actually has the permission to read or write (depending on the context) on the memory at the given pointer.

TODO: rework how the kernel checks memory access, then document it
