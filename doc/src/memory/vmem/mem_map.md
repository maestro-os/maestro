# Memory map

The memory space of each process is divided into several chunks. Those chunks are described for each architecture in the following sections.



## x86 (32 bits)

| Begin        | End          | Description                                                                                             |
|--------------|--------------|---------------------------------------------------------------------------------------------------------|
| `0x00000000` | `0x00001000` | The first page is not mapped in order to make the program crash in case of access to the `NULL` pointer |
| `0x00001000` | `0x40000000` | ELF image                                                                                               |
| `0x40000000` | `0xc0000000` | Allocatable memory (including stacks)                                                                   |
| `0xc0000000` | end          | Kernel space (not accessible directly by the program, except the `vDSO`)                                |
