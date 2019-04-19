#include "util.h"

#define UNIT_SIZE		   	(sizeof(char))
#define UNIT(bitmap, index)	(bitmap + (index / UNIT_SIZE))
#define INNER_INDEX(index)	(UNIT_SIZE - (index % UNIT_SIZE) - 1)

#define IS_ALIGNED(ptr)	(((intptr_t) (ptr) & (sizeof(long) - 1)) == 0)

#define RIGHT_MASK(out, mask_type, size)\
	out = 0;\
	for(size_t i = 0; i < (size); ++i)\
	{\
		out |= 0b1;\
		out <<= 1;\
	}
# define LEFT_MASK(out, mask_type, size)\
	out = 0;\
	for(size_t i = 0; i < (size); ++i)\
	{\
		out |= 0b1 << (BIT_SIZEOF(mask_type) - 1);\
		out >>= 1;\
	}

__attribute__((hot))
int bitmap_get(char *bitmap, const size_t index)
{
	return (*UNIT(bitmap, index) >> INNER_INDEX(index)) & 0b1;
}

__attribute__((hot))
void bitmap_set(char *bitmap, const size_t index)
{
	*UNIT(bitmap, index) |= (0b1 << INNER_INDEX(index));
}

__attribute__((hot))
void bitmap_clear(char *bitmap, const size_t index)
{
	*UNIT(bitmap, index) &= ~(0b1 << INNER_INDEX(index));
}

__attribute__((hot))
void bitmap_set_range(char *bitmap, const size_t begin, const size_t end)
{
	long mask;
	const char tiny_mask = ~((char) 0);
	size_t i = begin / BIT_SIZEOF(*bitmap);

	if(begin % UNIT_SIZE != 0)
	{
		RIGHT_MASK(mask, MASK, UNIT_SIZE - INNER_INDEX(begin));
		*UNIT(bitmap, begin) |= mask;
		++i;
	}

	if((end - begin) / 8 >= sizeof(mask))
	{
		while((i + sizeof(tiny_mask)) * 8 < end && !IS_ALIGNED(bitmap + i))
		{
			*UNIT(bitmap, i) = tiny_mask;
			i += sizeof(tiny_mask);
		}

		mask = ~((long) 0);

		while((i + sizeof(mask)) * 8 < end)
		{
			*((long *) UNIT(bitmap, i)) = mask;
			i += sizeof(mask);
		}
	}

	while((i + sizeof(tiny_mask)) * 8 < end)
	{
		*UNIT(bitmap, i) = tiny_mask;
		i += sizeof(tiny_mask);
	}

	if(end % UNIT_SIZE != 0)
	{
		LEFT_MASK(mask, mask, INNER_INDEX(end));
		*UNIT(bitmap, begin) |= mask;
	}
}

__attribute__((hot))
void bitmap_clear_range(char *bitmap, const size_t begin, const size_t end)
{
	long mask;
	size_t i = begin / BIT_SIZEOF(*bitmap);

	if(begin % UNIT_SIZE != 0)
	{
		RIGHT_MASK(mask, MASK, UNIT_SIZE - INNER_INDEX(begin));
		*UNIT(bitmap, begin) &= ~mask;
		++i;
	}

	if((end - begin) / 8 >= sizeof(mask))
	{
		while((i + sizeof(*bitmap)) * 8 < end && !IS_ALIGNED(bitmap + i))
		{
			*UNIT(bitmap, i) = 0;
			i += sizeof(*bitmap);
		}

		while((i + sizeof(mask)) * 8 < end)
		{
			*((long *) UNIT(bitmap, i)) = 0;
			i += sizeof(mask);
		}
	}

	while((i + sizeof(*bitmap)) * 8 < end)
	{
		*UNIT(bitmap, i) = 0;
		i += sizeof(*bitmap);
	}

	if(end % UNIT_SIZE != 0)
	{
		LEFT_MASK(mask, mask, INNER_INDEX(end));
		*UNIT(bitmap, begin) &= ~mask;
	}
}
