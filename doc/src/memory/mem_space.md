# Memory space

A memory space is a virtual memory context on which reside one or more process. It allows isolation of processes from each others.

Some processes may share the same memory space, for example using the `clone` system call.

A memory space contains the following components:
- Memory mapping: a region of virtual memory in use
- Memory gap: a region of virtual memory which is free, ready for allocations

A process can interact with its memory space using system calls such as `mmap`, `munmap`, `mlock`, `munlock` and `mprotect`.



## Lazy allocations

A memory mapping is supposed to point to a physical memory in order to work properly. However, allocating physical memory directly when the memory mapping is created or cloned takes significant resources that might not be used.

For example, when using the `fork` system call, the whole memory space has to be duplicated, often to be quickly followed by a call to `execve`, which removes the whole memory space.

To prevent this problem, physcial memory is allocated lazily.

To do so, the kernel maps the virtual memory in read-only. Then, when an attempt to modify the memory (write) occurs, the CPU triggers a page fault, which can then be handled by the kernel to make the actual allocation

The following cases can occur:
- simple allocation (example: `mmap`): The virtual memory is mapped to a default page which contains only zeros. When the kernel receives a page fault for this mapping, it allocates a new physcial page and maps it at the appropriate location
- duplication (example: `fork`): The virtual memory of the new memory space is mapped to the same physical memory as the original. Then writing is disabled on both. When a page fault is received, the kernel performs the same operation as the previous point, except the data present on the page is also copied.

Once the allocation has been made, the kernel enables writing permission on the mapping, then resume the execution. This procedure is totally transparent from the process's point of view.
