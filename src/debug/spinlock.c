#include <kernel.h>
#include <debug/debug.h>
#include <memory/memory.h>
#include <util/util.h>

#ifdef spin_lock
# undef spin_lock
#endif
#ifdef spin_unlock
# undef spin_unlock
#endif

static void invalid_spinlock(spinlock_t *spinlock)
{
	printf("INVALID SPINLOCK ADDRESS `%p`!\n", spinlock);
	kernel_halt();
}

void debug_spin_lock(spinlock_t *spinlock,
	const char *file, const size_t line)
{
	printf("Spin locked %p in %s at line %zu\n", spinlock, file, line);
	if((void *) spinlock < KERNEL_BEGIN)
		invalid_spinlock(spinlock);
	spin_lock(spinlock);
}

void debug_spin_unlock(spinlock_t *spinlock,
	const char *file, const size_t line)
{
	printf("Spin unlocked %p in %s at line %zu\n", spinlock, file, line);
	if((void *) spinlock < KERNEL_BEGIN)
		invalid_spinlock(spinlock);
	spin_unlock(spinlock);
}
