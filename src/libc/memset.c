#include "string.h"

// TODO Rewrite
void* memset(void* s, const int c, size_t n)
{
	for(size_t i = 0; i < n; ++i) *((char*) s + i) = c;

	return s;
}
