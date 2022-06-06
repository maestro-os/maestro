Memory space
************

A memory space is a virtual memory space which allow a process to have its own isolated environement.
Each processes virtually have the whole memory space available, although it shall not be used entirely as several process have to share the same physical memory.

The memory space is divided into two types of chunks:

- **Mappings**: Used memory space, virtually allocated although it might not have a dedicated physical memory
- **Gaps**: Virtual memory available for allocation

To manipulate the virtual memory, a memory space wraps a vmem object.

For memory space API references, check the documentation of module `process::mem_space`.



Mapping
-------

When a new mapping is allocated, no physical memory is linked to it. The default mapping is to a read only, zero-ed page that is used to fake the allocation.
When the process first tries to write to the page, an exeception is triggered because the page is mapped in read-only. This exception allows the kernel to allocate a new physical page and to map it to the virtual page. Then the process resumes and can continue its operations on the page like nothing happened.

A mapping can have shared physical memory with another mapping. This is especially usefull for sharing data with other processes.
Each allocated physical page has a counter associated with it allowing to know how many mappings point to it.

The kernel stack is *not* lazy-allocated, for a simple reason: When a memory page is written for the first time, an exception is triggered. However the kernel stack is required in order to handle an exception.
On x86, if the kernel stack itself is not mapped, the CPU shall trigger a double fault which in turn shall lead to a triple fault and reboot the system.



Gap
---

A gap is a mapping of memory which is available for allocation.
Default gaps, which indicate which portion of the virtual memory can be allocated in the beginning must exclude:

- The first page of memory, which must not be allocated to ensure that accessing the NULL pointer is invalid
- The kernel stub, to allow access to the code to execute system calls
- The last page, to ensure that buffer overflow do not result in pointer overflow



COW
===

COW (Copy-On-Write) is a feature allowing to lazy-allocate physical pages upon process fork.

When a process is forked, its memory space has the be duplicated for the newly created process. Although they both have the same virtual memory, they must not share the same physical memory.
Pages allocation and copy is expensive. Moreover, most fork operations are followed by program execution, which replaces the memory space with a new one. So most of the time, allocations are useless.

To counteract this problem, the kernel maps both memory mappings to the same physical memory in read only.
When a process first attempts to write to a page, a physical page is allocated, the data is copied and it and the link between virtual pages of the two memory spaces is removed.
