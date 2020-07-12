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

static void invalid_spinlock(spinlock_t *spinlock)
{
	void *ebp;

	printf("DEBUG: Invalid spinlock address `%p`\n", spinlock);
	GET_EBP(ebp);
	print_callstack(ebp, 8);
	kernel_halt();
}

void debug_spin_lock(spinlock_t *spinlock,
	const char *file, const size_t line)
{
	printf("DEBUG: Spin locked %p in %s at line %zu\n", spinlock, file, line);
	if((void *) spinlock < KERNEL_BEGIN
		|| (void *) spinlock >= mem_info.memory_end)
		invalid_spinlock(spinlock);
	spin_lock(spinlock);
}

void debug_spin_unlock(spinlock_t *spinlock,
	const char *file, const size_t line)
{
	printf("DEBUG: Spin unlocked %p in %s at line %zu\n", spinlock, file, line);
	if((void *) spinlock < KERNEL_BEGIN
		|| (void *) spinlock >= mem_info.memory_end)
		invalid_spinlock(spinlock);
	spin_unlock(spinlock);
}
