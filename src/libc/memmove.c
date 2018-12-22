#include "string.h"

void* memmove(void* dest, const void* src, size_t n)
{
	size_t i;

	if((uintptr_t) dest % sizeof(long) == 0
		&& (uintptr_t) src % sizeof(long) == 0
		&& n % sizeof(long) == 0) {
		if(dest < src) {
			i = 0;

			while(i < n) {
				*((long*) dest + i) = *((long*) src + i);
				++i;
			}
		} else {
			i = n;

			do {
				*((long*) dest + (i - sizeof(long)))
					= *((long*) src + (i - sizeof(long)));
				i -= sizeof(long);
			} while(i != 0);
		}
	} else {
		if(dest < src) {
			i = 0;

			while(i < n) {
				*((char*) dest + i) = *((char*) src + i);
				++i;
			}
		} else {
			i = n;

			do {
				*((char*) dest + (i - sizeof(char)))
					= *((char*) src + (i - sizeof(char)));
				i -= sizeof(long);
			} while(i != 0);
		}
	}

	return dest;
}
