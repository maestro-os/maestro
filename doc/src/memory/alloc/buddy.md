# Buddy allocator

A buddy allocator allows to allocate pages of memory by recursively dividing a big chunk of memory into smaller chunks.

Such an allocator is used by the kernel to provide memory pages for both kernelspace and userspace.

Detailed description of the allocator is available on [Wikipedia](https://en.wikipedia.org/wiki/Buddy_memory_allocation).
