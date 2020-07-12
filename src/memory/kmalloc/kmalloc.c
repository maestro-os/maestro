#include <memory/kmalloc/kmalloc.h>
#include <memory/kmalloc/kmalloc_internal.h>
#include <util/util.h>

#include <libc/errno.h>

/*
 * Allocates a block of memory of the given size in bytes and returns the
 * pointer to the beginning of it.
 * The block of memory can later be freed using `kfree`.
 *
 * On fail the function returns `NULL` and sets the errno to ENOMEM.
 */
ATTR_MALLOC
void *kmalloc(size_t size)
{
	void *ptr;

	if(size == 0)
		return NULL;
	spin_lock(&kmalloc_spinlock);
	ptr = alloc(size);
	spin_unlock(&kmalloc_spinlock);
	if(!ptr)
		errno = ENOMEM;
	return ptr;
}

/*
 * Calls `kmalloc` with the given size, clears the block of memory and returns
 * the pointer.
 */
void *kmalloc_zero(size_t size)
{
	void *ptr;

	if(likely(ptr = kmalloc(size)))
		bzero(ptr, size);
	return ptr;
}
