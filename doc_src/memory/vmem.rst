VMem
****

VMem (abreviation of Virtual Memory) is an object allowing to manipulate virtual memory paging.

Each CPU architecture requires its own implementation of paging since they all have a different way of handling it.

For vmem API references, check the documentation of module `memory::vmem`.



Binding
-------

Binding is the action of linking a vmem so that it applies to the current context.



Cache considerations
--------------------

Under some architectures, virtual memory entries are mapped into a special cache (under x86, this cache is called Translation Lookaside Buffer, or TLB for short).
Binding a vmem has the effect of flushing this cache (also called "TLB shootdown"). This is a slow operation and therefore it must be avoided as much as possible.

However, when modifying the mapping of the virtual memory, a TLB shootdown or page invalidation is necessary to ensure the cache doesn't contain invalid informations.

A page invalidation allows to refresh a single page in the TLB without flushing the whole cache.

Memory pages themselves also have caches (L1, L2, L3) to allow faster access to the most used pages. However they come with a few drawbacks:

- If the kernel has to perform MMIO on a memory page, it has to ensure caching is disabled it
- Caches might not be synchronized between CPU cores. The kernel has to send an interruption to other cores to flush their cache if necessary
