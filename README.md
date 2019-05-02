# CrumbleOS

CrumbleOS is a simple OS created just for fun.



## Kernel overview

The kernel is a homemade UNIX-like kernel.
Its features main features are the following:
- Memory management
- VGA display
- PS/2 controller management
- Multitasking & processes 
- Syscalls
- Filesystems
- Sound & PC Speaker
- Time
- etc...



### Memory management

#### Buddy allocator

Physical pages allocation is done using a buddy allocator which works by dividing blocks of memory in halves in order to get a block large enough.
The buddy allocator can allocate blocks of 2^n pages. This allows fast allocation, however it produces some external fragmentation.

When the kernel needs a block of memory, it first checks the free list.
The free list is an array of linked list which contains free blocks for a given order.

The free list keeps an array of orders up to ``FREE_LIST_MAX_ORDER``.
When a block is split and not used, if its order is small enough to fit into the free list, the kernel adds it.



When freeing a block, the kernel checks if it buddy is also free. If it is, then they are merged.
The kernel does this for the parents recursively until it can't merge blocks.



#### Slab allocator

The slab allocator is initialized after the buddy allocator.
It preallocates zones of memory (called slabs) containing caches that will keep kernels allocations.

TODO



#### Virtual memory

TODO



#### Pages swaping

TODO



### VGA display

#### TTY

TODO



#### Video mode

TODO



### PS/2 controller

#### Keyboard

TODO



#### Mouse

TODO



### Multitasking & processes

TODO



### Syscalls

TODO



### Filesystems

TODO



### Sound & PC speaker

TODO



#### PC speaker

TODO



### Time

TODO
