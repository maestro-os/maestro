#include <kernel.h>
#include <ssp/ssp.h>

__attribute__((noreturn))
void __stack_chk_fail(void)
{
	PANIC("Stack smashing detected!", 0);
}
