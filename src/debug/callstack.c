#include <debug/debug.h>

__attribute__((cold))
static const char *get_function_name(void *inst)
{
	// TODO
	(void) inst;
	return "TODO";
}

__attribute__((cold))
void print_callstack(void *ebp, const size_t max_depth)
{
	size_t i = 0;
	void *eip;

	printf("--- Callstack ---\n");
	while(ebp && i < max_depth)
	{
		eip = (void *) (*(int *) (ebp + 4));
		// TODO Use %zu
		printf("%i: %p -> %s\n", (int) i, eip, get_function_name(eip));
		ebp = (void *) (*(int *) ebp);
		++i;
	}
	if(ebp)
		printf("...\n");
}
