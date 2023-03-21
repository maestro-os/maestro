# malloc

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



## Safe interface

It is recommended to use the safe interface through the `Alloc` structure instead of the low-level functions described above.
