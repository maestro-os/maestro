# Userspace

The userspace is the place where user programs run. It is meant to have an interface close to Linux.

A program running in userspace is initialized using the **System V ABI** specification (see [external documentation](../external_doc.md)).

## System call

System calls are the main way for programs to communicate with the kernel.

The system call table can be found at the root of the module `kernel::syscall`.

### ABI by architecture

**Note**: the **end** bound of errno ranges are exclusive

#### x86 (32 bits)

|              |                                          |
|--------------|------------------------------------------|
| Instruction  | `int 0x80`                               |
| Syscall ID   | `eax`                                    |
| Arguments    | `ebx`, `ecx`, `edx`, `esi`, `edi`, `ebp` |
| Return value | `eax`                                    |
| Errno range  | `-4095..0`                               |

#### x86_64

|              |                                        |
|--------------|----------------------------------------|
| Instruction  | `int 0x80`, `syscall`                  |
| Syscall ID   | `rax`                                  |
| Arguments    | `rdi`, `rsi`, `rdx`, `r10`, `r8`, `r9` |
| Return value | `rax`                                  |
| Errno range  | `-4095..0`                             |

## Compatibility mode

The kernel supports running 32-bit programs on 64-bit kernels. The ABI is the same as kernels compiled for 32-bit.
