#ifndef _LIB_INTERNAL_H
# define _LIB_INTERNAL_H

__attribute__((hot))
__attribute__((inline))
__attribute__((const))
inline long make_field(const int c)
{
	long field = 0;
	for(size_t i = 0; i < sizeof(long); ++i)
		field = (field << 1) | (c & 0xff);

	return field;
}

#endif
