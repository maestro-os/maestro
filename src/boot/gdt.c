#include <stddef.h>
#include <stdint.h>

/*
 * Physical address to the GDT.
 */
# define GDT_PHYS_PTR	((void *) 0x800)
/*
 * Virtual address to the GDT.
 */
# define GDT_VIRT_PTR	((void *) 0xc0000000 + (uintptr_t) GDT_PHYS_PTR)
/*
 * Physical address to the GDT descriptor.
 */
# define GDT_DESC_PHYS_PTR	(GDT_PHYS_PTR\
	+ ((uintptr_t) &gdt - (uintptr_t) &gdt_start))
/*
 * Virtual address to the GDT descriptor.
 */
# define GDT_DESC_VIRT_PTR	((void *) 0xc0000000\
	+ (uintptr_t) GDT_DESC_PHYS_PTR)

/*
 * The size of the GDT in bytes, including the descriptor.
 */
# define GDT_SIZE	((uintptr_t) &gdt - (uintptr_t) &gdt_start\
	+ sizeof(gdt_descriptor_t))

/*
 * Structure representing a GDT entry.
 */
__attribute__((packed))
struct gdt_entry
{
	uint16_t limit_low;
	uint16_t base_low;
	uint8_t base_mid;
	uint8_t access;
	uint8_t flags_limit;
	uint8_t base_high;
};

/*
 * Structure representing the GDT descriptor.
 */
__attribute__((packed))
struct gdt_descriptor
{
	uint16_t size;
	uint32_t offset;
};

typedef struct gdt_entry gdt_entry_t;
typedef struct gdt_descriptor gdt_descriptor_t;

/*
 * The symbol to the GDT before relocation.
 */
extern gdt_entry_t gdt_start;
/*
 * The symbol to the GDT descriptor before relocation.
 */
extern gdt_descriptor_t gdt;

/*
 * Copies the GDT to its physical address.
 */
__attribute__((section(".boot.text")))
void gdt_copy(void)
{
	const char *src;
	char *dest;
	size_t i = 0, len;

	src = (const char *) &gdt_start;
	dest = GDT_PHYS_PTR;
	len = GDT_SIZE;
	while(i < len)
	{
		dest[i] = src[i];
		++i;
	}
}
