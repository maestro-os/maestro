#ifndef KERNEL_H
# define KERNEL_H

# include "libc/string.h"

typedef struct gdt_descriptor
{
	uint16_t size;
	uint32_t offset;
} gdt_descriptor_t;

typedef struct gdt_table
{
	// TODO
} gdt_table_t;

__attribute__((noreturn))
void panic(const char* reason);

__attribute__((noreturn))
void abort();

__attribute__((noreturn))
void kernel_halt();

uint8_t inb(const uint16_t port);
void outb(const uint16_t port, const uint8_t value);

#endif
