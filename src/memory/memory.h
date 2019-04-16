#ifndef MEMORY_H
# define MEMORY_H

# include "../kernel.h"

# define PAGE_SIZE	0x1000

# define HEAP_BEGIN ((void *) 0x400000)

typedef uint16_t paging_flags_t;
typedef uint8_t kmalloc_flags_t;

void *memory_end;

extern bool check_a20();
void enable_a20();

void physical_init();
size_t physical_available();
void *physical_alloc();
bool physical_alloc2(size_t pages, void (*f)(void *, void *), void *handle);
void physical_free(void *ptr);

inline void *page_to_ptr(const size_t page)
{
	return (void *) (page * PAGE_SIZE);
}

inline size_t ptr_to_page(const void *ptr)
{
	return (uintptr_t) ptr / PAGE_SIZE;
}

bool paging_is_allocated(const uint32_t *directory,
	const void *ptr, const size_t length);
void *paging_alloc(uint32_t *directory, void *hint,
	const size_t length, const paging_flags_t flags);
void paging_free(uint32_t *directory, void *ptr, const size_t length);
uint32_t *paging_get_page(const uint32_t *directory, const size_t page);
void paging_set_page(uint32_t *directory, const size_t page,
	void *physaddr, const paging_flags_t flags);

extern void paging_enable(const uint32_t *directory);
extern void paging_disable();

void *kmalloc(const size_t size);
void *krealloc(void *ptr, const size_t size);
void kfree(void *ptr);

// TODO Processes memory

#endif
