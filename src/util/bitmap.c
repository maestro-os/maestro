#include <util/util.h>
#include <memory/memory.h>

#define UNIT_SIZE		   		(BIT_SIZEOF(uint8_t))
#define UNIT(bitfield, index)	((bitfield) + ((index) / UNIT_SIZE))
#define INNER_INDEX(index)		(UNIT_SIZE - ((index) % UNIT_SIZE) - 1)

// TODO Protect and clean every function

ATTR_HOT
int bitfield_get(const uint8_t *bitfield, const size_t index)
{
	return (*UNIT(bitfield, index) >> INNER_INDEX(index)) & 0b1;
}

ATTR_HOT
void bitfield_set(uint8_t *bitfield, const size_t index)
{
	*UNIT(bitfield, index) |= (0b1 << INNER_INDEX(index));
}

ATTR_HOT
void bitfield_clear(uint8_t *bitfield, const size_t index)
{
	*UNIT(bitfield, index) &= ~(0b1 << INNER_INDEX(index));
}

ATTR_HOT
void bitfield_toggle(uint8_t *bitfield, const size_t index)
{
	if(bitfield_get(bitfield, index))
		bitfield_clear(bitfield, index);
	else
		bitfield_set(bitfield, index);
}

ATTR_HOT
void bitfield_set_range(uint8_t *bitfield, const size_t begin, const size_t end)
{
	size_t i;

	for(i = begin; i < end; ++i)
		bitfield_set(bitfield, i);
}

ATTR_HOT
void bitfield_clear_range(uint8_t *bitfield,
	const size_t begin, const size_t end)
{
	size_t i;

	for(i = begin; i < end; ++i)
		bitfield_clear(bitfield, i);
}

ATTR_HOT
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
