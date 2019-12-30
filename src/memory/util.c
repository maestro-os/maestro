#include <memory/memory.h>
#include <libc/stdio.h>

static const char *units[] = {
	"KB", "MB", "GB", "TB", "EB", "ZB", "YB"
};

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

ATTR_HOT
void *clone_page(void *ptr)
{
	void *new_page;

	ptr = (void *) ((uintptr_t) ptr & PAGING_ADDR_MASK);
	if((new_page = buddy_alloc(0)))
		memcpy(new_page, ptr, PAGE_SIZE);
	return new_page;
}
