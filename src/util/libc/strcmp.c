#include <stdint.h>
#include <stddef.h>

#include "libc.h"

/*
 * Compares two strings `s1` and `s2` and returns the diffence between the first characters that differ.
 */
int strcmp(const char *s1, const char *s2)
{
	// The index of the current byte
	size_t i;
	// The length of `s1`
	size_t l1;
	// The length of `s2`
	size_t l2;

	i = 0;
	l1 = strlen(s1);
	l2 = strlen(s2);
	while (i < l1 && i < l2 && s1[i] == s2[i])
		++i;
	return (((unsigned char *) s1)[i] - ((unsigned char *) s2)[i]);
}
