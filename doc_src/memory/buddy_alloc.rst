Physical allocation
*******************

Physical memory allocation is performed using a Buddy Allocator, which allows to allocate chunks of memory by recursively splitting bigger blocks in half.


Overview
========

The buddy allocator is the main memory allocator for the kernel, giving memory to every others allocator.

On initialization, the allocator owns a block of **n** pages of 4096 bytes of memory. The amount **n** of pages is determined by the available memory on the system.
The first step is to split this block so that only blocks of size **2^^n** pages are present. This can be done through decomposition into factors of **2**.

The buddy allocator can only allocate blocks of **2^n** pages, where **n** is called the *order* of the block.

The buddy allocator API exposes several functions, see the related documentation (TODO: Insert link to functions documentation) for more informations.



Allocation
==========

A free list is used by the allocator to access blocks of the requested size in constant time.

This free list (see diagram below) stores free memory blocks by order. Every locations in the array contains a linked list of free blocks.

TODO: Insert diagram of the free list

At the beginning, the free list only contains initial blocks of memory.

If a block of order **n** is requested, the allocator checks index **n** of the list. If the location is empty, it checks **n + 1**, and then **n + 2**, etc... until it finds a free block.

If not block is found, the system is considered to be out of memory and the allocation shall not be fullfilled.

If the block is larger than the requested size, it shall be splitted until it reaches the requested size.
When splitting a block, the order is decremented and two new free blocks of the same order, are created. Every create block shall be inserted into the free list in order to serve for further allocations.



Freeing
=======

Freeing memory is similar to allocation, except that the process is reversed.

The buddy block of a block is the block whom shall be coalesced with. The address of this block can be computed using the following formula:

::

	(addr - buddy_begin) ^ size

Where,

- **addr** is the address of the block
- **buddy_begin** is the pointer to the beginning of the buddy block
- **size** is the size of the block in bytes

Note: the **buddy_begin** variable must be the beginning of the block which was split at initialization of the allocator, and thus might not be the very beginning of the buddy allocation zone.

When freeing a block of memory, the allocator checks if the buddy block is free. If so, the two blocks are coalesced. This has the effect to increment the order of the new block.
This operation is repeated as much as possible. The block order cannot exceed the maximum block order.
