#include <debug/debug.h>
#include <kernel.h>
#include <memory/memory.h>
#include <util/util.h>

#include <libc/stdio.h>

#ifdef spin_lock
# undef spin_lock
#endif
#ifdef spin_unlock
# undef spin_unlock
#endif

void debug_spin_lock(spinlock_t *spinlock,
	const char *file, const size_t line)
{
	printf("DEBUG: Spin locked %p in %s at line %zu\n", spinlock, file, line);
	spin_lock(sanity_check(spinlock));
}

void debug_spin_unlock(spinlock_t *spinlock,
	const char *file, const size_t line)
{
	printf("DEBUG: Spin unlocked %p in %s at line %zu\n", spinlock, file, line);
	spin_unlock(sanity_check(spinlock));
}
