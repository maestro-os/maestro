#include <kernel.h>
#include <debug/debug.h>
#include <util/util.h>
#include <memory/memory.h>

ATTR_BSS
static profiler_func_t *funcs;

static void profiler_sort(void)
{
	profiler_func_t *f0, *f1;
	const char *name;
	size_t count;

	f0 = funcs;
	while(f0)
	{
		f1 = f0->next;
		while(f1)
		{
			if(f0->count < f1->count)
			{
				name = f0->name;
				count = f0->count;
				f0->name = f1->name;
				f0->count = f1->count;
				f1->name = name;
				f1->count = count;
			}
			f1 = f1->next;
		}
		f0 = f0->next;
	}
}

static void profiler_increment(const char *name)
{
	profiler_func_t *f;

	f = funcs;
	while(f)
	{
		if(strcmp(f->name, name) == 0)
			break;
		f = f->next;
	}
	if(!f)
	{
		if(!(f = kmalloc_zero(sizeof(funcs))))
			PANIC("Profiler memory allocation failed!", 0);
		f->name = name;
		f->next = funcs;
		funcs = f;
	}
	++f->count;
	profiler_sort();
}

void profiler_capture(void)
{
	size_t i = 0;
	void *ebp, *eip;
	const char *name;

	GET_EBP(ebp);
	while(ebp && i++ < 128)
	{
		if(!(eip = (void *) (*(intptr_t *) (ebp + 4))))
			break;
		if(!(name = get_function_name(eip)))
			name = "UNKNOWN";
		profiler_increment(name);
		ebp = (void *) (*(intptr_t *) ebp);
	}
}

void profiler_print(void)
{
	profiler_func_t *f;

	printf("--- Profiler ---\n");
	f = funcs;
	while(f)
	{
		printf("-> %s %zu\n", f->name, f->count);
		f = f->next;
	}
}
