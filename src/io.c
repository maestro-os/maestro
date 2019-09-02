#include <kernel.h>

__attribute__((hot))
uint8_t inb(const uint16_t port)
{
	uint8_t ret;
	asm volatile("inb %1, %0" : "=a"(ret) : "d"(port));

	return ret;
}

__attribute__((hot))
uint16_t inw(const uint16_t port)
{
	uint16_t ret;
	asm volatile("inw %1, %0" : "=a"(ret) : "d"(port));

	return ret;
}

__attribute__((hot))
uint32_t inl(uint16_t port)
{
	uint32_t ret;
	asm volatile("inl %1, %0" : "=a"(ret) : "d"(port));

	return ret;
}

__attribute__((hot))
void outb(const uint16_t port, const uint8_t value)
{
	asm volatile("outb %0, %1" : : "a"(value), "d"(port));
}

__attribute__((hot))
void outw(const uint16_t port, const uint16_t value)
{
	asm volatile("outw %0, %1" : : "a"(value), "d"(port));
}

__attribute__((hot))
void outl(const uint16_t port, const uint32_t value)
{
	asm volatile("outl %0, %1" : : "a"(value), "d"(port));
}
