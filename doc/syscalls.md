# System calls

System calls allow processes to interact with the kernel.
For example, they can allow to:
- Open and read/write files
- Create processes
- Use the filesystem
- Communicate with the outside
- etc...

To perform a syscall, a process must:
- Assign the syscall id into register `eax`
- Assign arguments into registers `ebx`, `ecx`, `edx`, `esi`, `edi` and `ebp` (in that order)
- Fire an interrupt on vector `0x80` with instruction: `int $0x80`
- Get the return code into register `eax`

If the return code of a syscall is negative, the errno must be set to the absolute value of the return code and return `-1`
