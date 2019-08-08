# Physical allocator

The physical allocator allows to get memory for every other parts of the kernel.
It works using a buddy allocator devised by Knowlton in **1965**.

Buddy allocators breaks up memory into blocks of pages where each block is a power of two number of pages.
If no block of the desired size is available, a larger block is broken up until reaching the good size.
If two buddies are free, they are coalesced up.



## Definitions

- **order**: The size of a block in form of a power of two number of pages (``2^order``)
- **buddy**: The sibling block needed to coalesce into a larger block



## Initialization

First, the allocator gets the maximum block order (``log2(pages)``).

Then, it reserves some space to store the state of blocks under the form of a binary tree.
The number of nodes in the tree equals ``2^(order + 1) - 1``.

The allocator works only if the size of the memory is a power of two, so if it's not the case we need to have a start block larger than the size of the memory and then mark every non-existing pages as full to prevent them from being allocated.

Finally, the allocators initializes a free list, which is an array containing pointers to the first element of linked lists for each block size.
Every free block is then registered in the corresponding linked list to provide fast access.



## Block state

The different blocks state are the following:
- **Free**: The block is free for allocation
- **Partial**: The block has been broken up
- **Full**: The block is used

When a block is allocated, it is marked as full.
When a block is freed, it is marked as free.

If the buddy block is free, it is added to the free list.
When a block is allocated and is present in the free list, it is removed from it.
When two blocks are merged, they are removed from the free list and the resulting block is added to it.

Each time a block changes state, its parent is updated:
- If both children are free, the block is marked as free
- If both children are full, the block is marked as full
- Else, the block is marked as partial



## Block searching

To find a free block, the allocator first looks into the free list to find a free block.

If no free block is found, the allocators look into the binary tree recursively, starting from the larger block:
- If the block is too small
	- No block available
- If the block is free
	- If the block is too large
		- The block is broken up
	- If the block has the good size
		- A free block is found
- If the block is partial
	- If block is larger than required size
		- Look into children blocks
- If the block is not the larger one and the allocator is not checking the buddy block
	- Check the buddy block
- Else, no block available
