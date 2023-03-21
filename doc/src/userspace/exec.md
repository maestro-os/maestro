# Userspace

The userspace is the space in which user programs run.

To ensure memory isolation between processes, each process has its own memory space.



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
