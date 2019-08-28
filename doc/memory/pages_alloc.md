# Pages allocator

The pages allocator allows to allocate a given amount of pages, as the buddy allocator can lead to a lot of fragmentation since it allocates `2^n` pages instead of `n` pages.
The allocator uses the unused pages allocated by the buddy allocator to allow further smaller allocations.



## Overview

To allocate pages, the pages allocator first checks if any already allocated block has enough space for the allocation.
If a block is found, then it is used for the allocation and mark as used.
If no block is found, the buddy allocator is called to get a block of memory large enough to contain the required number of pages.
If some pages remain unused, the alloctor puts them into a different block, marked as free.

Each block keeps a reference to the buddy block it was allocated on.
When pages are freed, the block is marked as free and is coalesced with adjacent blocks if also free.
If every pages of a buddy are freed, the buddy block is freed.

The pages alloctor has a free list which stores pointers to blocks. Each pointer corresponds to the first block of `2^n` pages or more, where `n` is the order of the amount of pages.
