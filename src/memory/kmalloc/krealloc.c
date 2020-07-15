#include <memory/kmalloc/kmalloc.h>
#include <memory/kmalloc/kmalloc_internal.h>
#include <util/util.h>

/*
 * Changes the size of the given memory region that was allocated with
 * `kmalloc`. Passing a NULL pointer is equivalent to calling `kmalloc` with the
 * specified `size`. If `ptr` is not NULL but `size` equals `0`, it is
 * equivalent to calling `kfree` with `ptr`.
 *
 * If the size of the region is increased, the newly allocated region is not
 * allocated.
 * If the region has to be moved, the data is copied to a new region of memory,
 * the old region is freed and the pointer to the beginning of the new region is
 * returned.
 */
void *krealloc(void *ptr, size_t size)
{
	if(!sanity_check(ptr))
		return kmalloc(size);
	if(size == 0)
	{
		kfree(ptr);
		return NULL;
	}
	spin_lock(&kmalloc_spinlock);
	// TODO
	spin_unlock(&kmalloc_spinlock);
	return NULL;
}
