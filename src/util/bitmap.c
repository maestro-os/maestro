#include <util/util.h>
#include <memory/memory.h>

#define UNIT_SIZE		   	(BIT_SIZEOF(uint8_t))
#define UNIT(bitfield, index)	((bitfield) + ((index) / UNIT_SIZE))
#define INNER_INDEX(index)	(UNIT_SIZE - ((index) % UNIT_SIZE) - 1)

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

// TODO Protect and clean every function

__attribute__((hot))
int bitfield_get(const uint8_t *bitfield, const size_t index)
{
	return (*UNIT(bitfield, index) >> INNER_INDEX(index)) & 0b1;
}

__attribute__((hot))
void bitfield_set(uint8_t *bitfield, const size_t index)
{
	*UNIT(bitfield, index) |= (0b1 << INNER_INDEX(index));
}

__attribute__((hot))
void bitfield_clear(uint8_t *bitfield, const size_t index)
{
	*UNIT(bitfield, index) &= ~(0b1 << INNER_INDEX(index));
}

__attribute__((hot))
void bitfield_toggle(uint8_t *bitfield, const size_t index)
{
	if(bitfield_get(bitfield, index))
		bitfield_clear(bitfield, index);
	else
		bitfield_set(bitfield, index);
}

__attribute__((hot))
void bitfield_set_range(uint8_t *bitfield, const size_t begin, const size_t end)
{
	long mask;
	const uint8_t tiny_mask = ~((uint8_t) 0);
	size_t i = begin / BIT_SIZEOF(*bitfield);

	if(begin % UNIT_SIZE != 0)
	{
		RIGHT_MASK(mask, MASK, UNIT_SIZE - INNER_INDEX(begin));
		*UNIT(bitfield, begin) |= mask;
		++i;
	}

	if((end - begin) / 8 >= sizeof(mask))
	{
		while((i + sizeof(tiny_mask)) * 8 < end
			&& !IS_ALIGNED(bitfield + i, PAGE_SIZE))
		{
			*UNIT(bitfield, i) = tiny_mask;
			i += sizeof(tiny_mask);
		}

		mask = ~((long) 0);

		while((i + sizeof(mask)) * 8 < end)
		{
			*((long *) UNIT(bitfield, i)) = mask;
			i += sizeof(mask);
		}
	}

	while((i + sizeof(tiny_mask)) * 8 < end)
	{
		*UNIT(bitfield, i) = tiny_mask;
		i += sizeof(tiny_mask);
	}

	if(end % UNIT_SIZE != 0)
	{
		LEFT_MASK(mask, mask, INNER_INDEX(end));
		*UNIT(bitfield, i) |= mask;
	}
}

__attribute__((hot))
void bitfield_clear_range(uint8_t *bitfield,
	const size_t begin, const size_t end)
{
	long mask;
	size_t i = begin / BIT_SIZEOF(*bitfield);

	if(begin % UNIT_SIZE != 0)
	{
		RIGHT_MASK(mask, MASK, UNIT_SIZE - INNER_INDEX(begin));
		*UNIT(bitfield, begin) &= ~mask;
		++i;
	}

	if((end - begin) / 8 >= sizeof(mask))
	{
		while((i + sizeof(*bitfield)) * 8 < end
			&& !IS_ALIGNED(bitfield + i, PAGE_SIZE))
		{
			*UNIT(bitfield, i) = 0;
			i += sizeof(*bitfield);
		}

		while((i + sizeof(mask)) * 8 < end)
		{
			*((long *) UNIT(bitfield, i)) = 0;
			i += sizeof(mask);
		}
	}

	while((i + sizeof(*bitfield)) * 8 < end)
	{
		*UNIT(bitfield, i) = 0;
		i += sizeof(*bitfield);
	}

	if(end % UNIT_SIZE != 0)
	{
		LEFT_MASK(mask, mask, INNER_INDEX(end));
		*UNIT(bitfield, i) &= ~mask;
	}
}

size_t bitfield_first_clear(const uint8_t *bitfield, const size_t bitfield_size)
{
	size_t i = 0;
	uint8_t c;
	size_t j = 0;

	while(i * UNIT_SIZE < bitfield_size && bitfield[i] == 0xff)
		++i;
	if(i * UNIT_SIZE >= bitfield_size)
		return bitfield_size;
	c = bitfield[i];
	while(c & (1 << 7) && i * UNIT_SIZE + j < bitfield_size)
	{
		c <<= 1;
		++j;
	}
	return i * UNIT_SIZE + j;
}
