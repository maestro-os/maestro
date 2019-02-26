#include "kernel.h"

void mm_init()
{
	mem_node_t *begin = KERNEL_HEAP_BEGIN;
	begin->state = MEM_FREE;
	begin->size = KERNEL_HEAP_SIZE - sizeof(mem_node_t);
	begin->next = NULL;
}

void *mm_find_free(void *ptr, size_t size)
{
	if(!size) return NULL;

	mem_node_t *n = KERNEL_HEAP_BEGIN;
	// TODO Realloc
	(void)ptr;

	while(n)
	{
		if(n->state != MEM_FREE || n->size < size)
		{
			n = n->next;
			continue;
		}

		if(n->size - size >= sizeof(mem_node_t)
			&& (void * )n + size + sizeof(mem_node_t) * 2 < memory_end)
		{
			mem_node_t *new_node = n + sizeof(mem_node_t) + size;
			new_node->state = MEM_FREE;
			new_node->size = n->size - size - sizeof(mem_node_t);

			new_node->next = n->next;
			n->next = new_node;
		}

		n->state = MEM_USED;
		n->size = size;
		return n + sizeof(mem_node_t);
	}

	return NULL;
}

static void merge_free_nodes()
{
	mem_node_t *n = KERNEL_HEAP_BEGIN;

	while(n->next)
	{
		if(n->state == MEM_FREE && n->next->state == MEM_FREE)
		{
			n->size = n->size + sizeof(mem_node_t) + n->next->size;
			n->next = n->next->next;
		}

		n = n->next;
	}
}

void mm_free(void *ptr)
{
	if(!ptr) return;

	mem_node_t *n = KERNEL_HEAP_BEGIN;

	while(n && n + sizeof(mem_node_t) != ptr)
	{
		n = n->next;
	}

	if(!n) return; // TODO Error? (not allocated)
	if(n->state == MEM_FREE) return; // TODO Error? (double free)

	merge_free_nodes();
}
