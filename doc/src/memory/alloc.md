# Allocators

This pagge describes memory allocators that are implemented inside of the kernel.



## Buddy allocator

The buddy allocator is the primary allocator which provides memory pages to all other allocators.

This allocator takes most of the memory on the system and works by recursively dividing them in two until a block of the required size is available.

Freeing memory works the other way around, by merging adjacent free blocks.

More details are available on [Wikipedia](https://en.wikipedia.org/wiki/Buddy_memory_allocation).

Since this allocator provides at least one page of memory per allocation, smaller objects need another allocator to subdivide pages into usable chunks. This is the role of **malloc**.



## malloc

The kernel has it's own version of the `malloc` function to allow memory allocations internal to the kernel.

The implementation is a located in `kernel::memory::malloc`.

Functions:
- `alloc`: Allocates the given amount of bytes and returns a pointer to the chunk
- `realloc`: Changes the size of an allocation
- `free`: Frees a previously allocated chunk

The allocator works using memory pages provided by the buddy allocator.

Allocated chunks are guaranteed to:
- Be accessible from kernelspace
- Not overlap with other chunks
- Be aligned in memory



### Safe interface

It is recommended to use the safe interface through the `Alloc` structure instead of the low-level functions described above.



## vmem

Virtual memory allows the kernel to provide each process with its own memory space, independent from other processes.

Refer to the documentation of the target CPU architecture for details on the way virtual memory works.
