#include <kernel.h>
#include <ssp/ssp.h>

ATTR_NORETURN
void __stack_chk_fail(void)
{
	PANIC("Stack smashing detected!", 0);
}
