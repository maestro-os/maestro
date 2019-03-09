#include "memory.h"

static global_descriptor_t create_gd(const uint32_t limit,
	const uint32_t base, const uint8_t access, const uint8_t flags)
{
	global_descriptor_t gd = GD_NULL;
	gd |= ((uint64_t) limit & GD_LIMIT_MASK) << GD_LIMIT_OFFSET;
	gd |= (((uint64_t) limit & GD_LIMIT_MASK_2)
		>> GD_LIMIT_SHIFT_2) << GD_LIMIT_OFFSET_2;

	gd |= ((uint64_t) base & GD_BASE_MASK) << GD_BASE_OFFSET;
	gd |= (((uint64_t) base & GD_BASE_MASK_2)
		>> GD_BASE_SHIFT_2) << GD_BASE_OFFSET_2;
	gd |= (((uint64_t) base & GD_BASE_MASK_3)
		>> GD_BASE_SHIFT_3) << GD_BASE_OFFSET_3;

	gd |= (uint64_t) access << GD_ACCESS_OFFSET;
	gd |= (uint64_t) flags << GD_FLAGS_OFFSET;

	return gd;
}

void create_gdt(gdt_t *gdt, global_descriptor_t *gdt_start)
{
	size_t i = 0;

	gdt_start[i++] = GD_NULL;
	create_gd(0x200, 0x100000, GD_ACCESS_BASE
		| GD_ACCESS_PRIVILEGE_RING_0 | GD_ACCESS_S | GD_ACCESS_EXECUTABLE,
			GD_FLAGS_GRANULARITY_4K | GD_FLAGS_SIZE_32BITS);
	create_gd(0x300, 0x200000, GD_ACCESS_BASE
		| GD_ACCESS_PRIVILEGE_RING_0 | GD_ACCESS_S
			| GD_ACCESS_DOWNWARD_EXPENSION | GD_ACCESS_WRITABLE,
			GD_FLAGS_GRANULARITY_4K | GD_FLAGS_SIZE_32BITS);
	// TODO TSS

	gdt->size = (i * sizeof(global_descriptor_t)) - 1;
}
