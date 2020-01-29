#ifndef PAGES_H
# define PAGES_H

# include <memory/memory.h>
# include <util/util.h>

void *pages_alloc(size_t n);
void *pages_alloc_zero(size_t n);
void pages_free(void *ptr, size_t pages);

#endif
