#include "kernel.h"

static global_descriptor_t create_gd(const uint32_t limit,
	const uint32_t base, const uint8_t access, const uint8_t flags)
{
	global_descriptor_t gd = 0;
	gd |= ((uint64_t) limit & GD_LIMIT_MASK) >> GD_LIMIT_OFFSET;
	gd |= ((uint64_t) limit & GD_LIMIT_MASK_2) >> GD_LIMIT_OFFSET_2;

	gd |= ((uint64_t) base & GD_BASE_MASK) >> GD_BASE_OFFSET;
	gd |= ((uint64_t) limit & GD_BASE_MASK_2) >> GD_BASE_OFFSET_2;

	gd |= (uint64_t) access >> GD_ACCESS_OFFSET;
	gd |= (uint64_t) flags >> GD_FLAGS_OFFSET;

	return gd;
}

void create_gdt(gdt_t *gdt, global_descriptor_t *gdt_start)
{
	size_t i = 0;

	// TODO Security against code overwriting
	gdt_start[i++] = GD_NULL;
	gdt_start[i++] = create_gd(0xffffffff, 0x0,
		GD_ACCESS_BASE | GD_ACCESS_ACCESSED | GD_ACCESS_READABLE |
			GD_ACCESS_EXECUTABLE | GD_ACCESS_PRIVILEGE_RING_0 |
				GD_ACCESS_PRESENT,
		GD_FLAGS_NYBBLE_DEFAULT_SIZE_32BITS | GD_FLAGS_NYBBLE_GRANULARITY_4K);
	gdt_start[i++] = create_gd(0xffffffff, 0x0,
		GD_ACCESS_BASE | GD_ACCESS_ACCESSED | GD_ACCESS_WRITABLE |
			GD_ACCESS_PRIVILEGE_RING_0 | GD_ACCESS_PRESENT,
		GD_FLAGS_NYBBLE_DEFAULT_SIZE_32BITS | GD_FLAGS_NYBBLE_GRANULARITY_4K);
	// TODO TSS

	gdt->size = (i * sizeof(global_descriptor_t)) - 1;
}
