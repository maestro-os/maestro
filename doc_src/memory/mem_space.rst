Memory space
************

A memory space is a virtual memory space which allow a process to have its own isolated environement.
Each processes virtually have the whole memory space available, although it shall not be used entirely as several process have to share the same physical memory.



Regions and gaps
================

The memory space is divided into two types of structures:

- **Regions**: Used memory space, virtually allocated although it might not have a dedicated physical memory
- **Gaps**: Virtual memory available for allocation



Region
------

A region is a memory mapping, when a new region is allocated, no physical memory is linked to it. The default mapping is to a read only, zero-ed page that is used to fake the allocation.
When the process first tries to write to the page, an exeception is triggered because the page is mapped in read-only. This exception allows the kernel to allocate a new physical page and to map it to the virtual page. Then the process resumes and can continue its operations on the page like nothing happened.

A region can have shared physical memory with another region. This is especially usefull for sharing data with another process.
If two or more regions of memory share the same physical page, they are linked together using a doubly linked-list, which allows to know if another region is using the physical memory.
The physical memory can be shared in read-only, has a special purpose (see section **COW**).

Stacks are also lazy-allocated, allowing to only allocate necessary memory for it. Stacks have a limit of memory, but most of the time, this limit is never reached. Thus it's useless to allocated the whole stack at once.

Kernelspace allocations for memory spaces are *NOT* lazy allocated, especially kernel stack, for a simple reason: When a memory page is written for the first time, an exception is trigger. However the kernel stack is required in order to handle an exception.
On x86, if the kernel stack itself is not mapped, the CPU shall trigger a double fault which shall lead to a triple fault and reset the CPU.



Gap
---

A gap is a region of memory which is available for allocation.
Default gaps must exclude some regions, including:

- The first page of memory, which must not be allocated to ensure that accessing the NULL pointer results in an invalid access
- The kernel stub, to allow access to the code to execute system calls
- The last page, to ensure that buffer overflow do not result in pointer overflow



Memory allocation
=================

When allocating memory on a memory space, the it is important to first find a gap large enough to fit the allocation.
This can be done in *O(log(n))* time thanks to AVL trees.

The gap can then be reduced in order leave room for the newly created region.



Memory freeing
==============

TODO



COW
===

COW (Copy-On-Write) is a feature allowing to lazy-allocate physical pages upon process fork.

When a process is forked, its memory space has the be duplicated for the newly created process. Although they both have the same virtual memory, they must not share the same physical memory.
Pages allocation and copy is expensive. Moreover, most fork operations are followed by program execution, which replaces the memory space with a new one. So most of the time, allocations are useless.

To counteract this problem, the kernel copies maps both memory regions to the same physical memory in read only.
When a process first attempts to write to a page, a physical page is allocated, the data is copied and it and the link between virtual pages of the two memory spaces is removed.
