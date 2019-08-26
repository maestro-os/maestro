#include <util/util.h>
#include <idt/idt.h>

void lock(spinlock_t *spinlock)
{
	// TODO CLI();
	// TODO spin_lock(spinlock);
	(void) spinlock;
}

void unlock(spinlock_t *spinlock)
{
	// TODO spin_unlock(spinlock);
	(void) spinlock;
	// TODO Do not restore `STI();` if it wasn't enabled in the first place
}
