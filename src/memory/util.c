#include <memory/memory.h>
#include <libc/stdio.h>

/*
 * The list of memory units.
 */
static const char *units[] = {
	"KB", "MB", "GB", "TB", "EB", "ZB", "YB"
};

/*
 * Prints the given amount of memory, formated with the right unit.
 */
void print_mem_amount(size_t amount)
{
	size_t n = 0;

	if(amount == 1)
	{
		printf("1 byte");
		return;
	}
	if(amount < 1024)
	{
		printf("%zu bytes", amount);
		return;
	}
	amount /= 1024;
	while(amount >= 1024 && n < sizeof(units) / sizeof(const char *))
	{
		amount /= 1024;
		++n;
	}
	printf("%zu %s", amount, units[n]);
}
