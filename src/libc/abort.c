#include "stdlib.h"

__attribute__((noreturn))
void abort()
{
	ABORT_INSTRUCTION;
	exit(127);

	while(1) ABORT_INSTRUCTION;
}
