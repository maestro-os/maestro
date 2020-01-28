#include <kernel.h>

/*
 * Inputs a byte from the specified port.
 */
ATTR_HOT
uint8_t inb(const uint16_t port)
{
	uint8_t ret;
	asm volatile("inb %1, %0" : "=a"(ret) : "d"(port));

	return ret;
}

/*
 * Inputs a word from the specified port.
 */
ATTR_HOT
uint16_t inw(const uint16_t port)
{
	uint16_t ret;
	asm volatile("inw %1, %0" : "=a"(ret) : "d"(port));

	return ret;
}

/*
 * Inputs a long from the specified port.
 */
ATTR_HOT
uint32_t inl(uint16_t port)
{
	uint32_t ret;
	asm volatile("inl %1, %0" : "=a"(ret) : "d"(port));

	return ret;
}

/*
 * Outputs a byte to the specified port.
 */
ATTR_HOT
void outb(const uint16_t port, const uint8_t value)
{
	asm volatile("outb %0, %1" : : "a"(value), "d"(port));
}

/*
 * Outputs a word to the specified port.
 */
ATTR_HOT
void outw(const uint16_t port, const uint16_t value)
{
	asm volatile("outw %0, %1" : : "a"(value), "d"(port));
}

/*
 * Outputs a long to the specified port.
 */
ATTR_HOT
void outl(const uint16_t port, const uint32_t value)
{
	asm volatile("outl %0, %1" : : "a"(value), "d"(port));
}
