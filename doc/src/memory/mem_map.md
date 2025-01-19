# Memory map

The memory space of each process is divided into several chunks. Those chunks are described for each architecture in the following sections.

## x86 (32-bit)

| Begin        | End          | Description                            |
|--------------|--------------|----------------------------------------|
| `0x00000000` | `0x00001000` | Not mapped, for the `NULL` pointer     |
| `0x00001000` | `0xc0000000` | Userspace (program image, stack, heap) |
| `0xc0000000` | end          | Kernel space                           |

## x86_64

| Begin                | End                  | Description                            |
|----------------------|----------------------|----------------------------------------|
| `0x0000000000000000` | `0x0000000000001000` | Not mapped, for the `NULL` pointer     |
| `0x0000000000001000` | `0x0000800000000000` | Userspace (program image, stack, heap) |
| `0x0000800000000000` | `0xffff800000000000` | Canonical hole, cannot be used         |
| `0xffff800000000000` | end                  | Kernel space                           |
