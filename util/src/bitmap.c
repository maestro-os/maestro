#include <util/util.h>
#include <memory/memory.h>

/*
 * Returns the size in bits of a bitfield unit.
 */
#define UNIT_SIZE		   		(BIT_SIZEOF(uint8_t))
/*
 * Retuns the unit for the given index.
 */
#define UNIT(bitfield, index)	((bitfield) + ((index) / UNIT_SIZE))
/*
 * Retuns the bit offset in the unit for the given index.
 */
#define INNER_INDEX(index)		(UNIT_SIZE - ((index) % UNIT_SIZE) - 1)

/*
 * This file handles bitfields.
 *
 * The usage of bitfields allows to store a large number of boolean values in a
 * limit space.
 * The bitfield is divided in units of at least one byte.
 */

// TODO Protect and clean every function

/*
 * Returns the value in the given bitfield at the given index.
 */
ATTR_HOT
int bitfield_get(const uint8_t *bitfield, const size_t index)
{
	return (*UNIT(bitfield, index) >> INNER_INDEX(index)) & 0b1;
}

/*
 * Sets the bit in the given bitfield at the given index.
 */
ATTR_HOT
void bitfield_set(uint8_t *bitfield, const size_t index)
{
	*UNIT(bitfield, index) |= (0b1 << INNER_INDEX(index));
}

/*
 * Clears the bit in the given bitfield at the given index.
 */
ATTR_HOT
void bitfield_clear(uint8_t *bitfield, const size_t index)
{
	*UNIT(bitfield, index) &= ~(0b1 << INNER_INDEX(index));
}

/*
 * Toggles the bit in the given bitfield at the given index.
 */
ATTR_HOT
void bitfield_toggle(uint8_t *bitfield, const size_t index)
{
	if(bitfield_get(bitfield, index))
		bitfield_clear(bitfield, index);
	else
		bitfield_set(bitfield, index);
}

/*
 * Sets bits in the given range in the given bitfield.
 */
ATTR_HOT
void bitfield_set_range(uint8_t *bitfield, const size_t begin, const size_t end)
{
	size_t i;

	for(i = begin; i < end; ++i)
		bitfield_set(bitfield, i);
}

/*
 * Clears bits in the given range in the given bitfield.
 */
ATTR_HOT
void bitfield_clear_range(uint8_t *bitfield,
	const size_t begin, const size_t end)
{
	size_t i;

	for(i = begin; i < end; ++i)
		bitfield_clear(bitfield, i);
}

/*
 * Returns the index of the first clear bit in the given bitfield.
 */
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
