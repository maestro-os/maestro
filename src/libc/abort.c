#include <libc/stdlib.h>

__attribute__((noreturn))
void abort(void)
{
	ABORT_INSTRUCTION;
	exit(127);

	while(1) ABORT_INSTRUCTION;
}
