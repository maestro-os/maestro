#include <util/util.h>
#include <memory/memory.h>

#define UNIT_SIZE		   	(sizeof(uint8_t))
#define UNIT(bitmap, index)	(bitmap + (index / UNIT_SIZE))
#define INNER_INDEX(index)	(UNIT_SIZE - (index % UNIT_SIZE) - 1)

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
int bitmap_get(uint8_t *bitmap, const size_t index)
{
	return (*UNIT(bitmap, index) >> INNER_INDEX(index)) & 0b1;
}

__attribute__((hot))
void bitmap_set(uint8_t *bitmap, const size_t index)
{
	*UNIT(bitmap, index) |= (0b1 << INNER_INDEX(index));
}

__attribute__((hot))
void bitmap_clear(uint8_t *bitmap, const size_t index)
{
	*UNIT(bitmap, index) &= ~(0b1 << INNER_INDEX(index));
}

__attribute__((hot))
void bitmap_toggle(uint8_t *bitmap, const size_t index)
{
	if(bitmap_get(bitmap, index))
		bitmap_clear(bitmap, index);
	else
		bitmap_set(bitmap, index);
}

__attribute__((hot))
void bitmap_set_range(uint8_t *bitmap, const size_t begin, const size_t end)
{
	long mask;
	const uint8_t tiny_mask = ~((uint8_t) 0);
	size_t i = begin / BIT_SIZEOF(*bitmap);

	if(begin % UNIT_SIZE != 0)
	{
		RIGHT_MASK(mask, MASK, UNIT_SIZE - INNER_INDEX(begin));
		*UNIT(bitmap, begin) |= mask;
		++i;
	}

	if((end - begin) / 8 >= sizeof(mask))
	{
		while((i + sizeof(tiny_mask)) * 8 < end
			&& !IS_ALIGNED(bitmap + i, PAGE_SIZE))
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
		*UNIT(bitmap, i) |= mask;
	}
}

__attribute__((hot))
void bitmap_clear_range(uint8_t *bitmap, const size_t begin, const size_t end)
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
		while((i + sizeof(*bitmap)) * 8 < end
			&& !IS_ALIGNED(bitmap + i, PAGE_SIZE))
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
		*UNIT(bitmap, i) &= ~mask;
	}
}

size_t bitmap_first_clear(uint8_t *bitmap, const size_t bitmap_size)
{
	size_t i = 0;
	while(i * UNIT_SIZE < bitmap_size && bitmap[i] == 0xff) ++i;

	if(i * UNIT_SIZE >= bitmap_size)
		return bitmap_size;

	uint8_t c = bitmap[i];
	size_t j = 0;
	while(c & (1 << 7))
	{
		c <<= 1;
		++j;
	}

	return i * UNIT_SIZE + j;
}
