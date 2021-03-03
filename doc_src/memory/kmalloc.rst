kmalloc
*******

The ``malloc`` module is a kernel side memory allocation utility which uses the buddy allocator to fullfill the kernel's internal memory allocation requirements. It works with a classical first-fit approache using a storage bins for lists of free memory chunks sorted by size.



TODO: Explain the way allocation/freeing work
