#include <stddef.h>
#include <stdint.h>

/*
 * The size in bytes of one page of memory.
 */
#define PAGE_SIZE	4096

/*
 * Paging flags.
 */
#define PAGING_FLAG_GLOBAL			0b100000000
#define PAGING_FLAG_PAGE_SIZE		0b010000000
#define PAGING_FLAG_DIRTY			0b001000000
#define PAGING_FLAG_ACCESSED		0b000100000
#define PAGING_FLAG_CACHE_DISABLE	0b000010000
#define PAGING_FLAG_WRITE_THROUGH	0b000001000
#define PAGING_FLAG_USER			0b000000100
#define PAGING_FLAG_WRITE			0b000000010
#define PAGING_FLAG_PRESENT			0b000000001

/*
 * This file handles kernel remapping in order to place it in High Memory.
 * To do so, paging is enabled using a page directory that remaps the whole
 * kernel.
 *
 * The created page directory has to be replaced when kernel memory management
 * is ready.
 */

/*
 * The page directory used for kernel remapping.
 */
__attribute__((aligned(PAGE_SIZE)))
__attribute__((section(".boot.data")))
static uint32_t remap_dir[1024];

extern void pse_enable(void *page_dir);
extern void gdt_move(void);

/*
 * Remaps the first gigabyte of memory to the last one. This function enables
 * PSE.
 * Note: the kernel can access the NULL pointer and write onto its own code
 * after this function. Thus the kernel must be protected as soon as possible.
 */
__attribute__((section(".boot.text")))
void kernel_remap(void)
{
	size_t i;
	const uint32_t flags = PAGING_FLAG_PAGE_SIZE | PAGING_FLAG_WRITE
		| PAGING_FLAG_PRESENT;
	uint32_t entry;

	for(i = 0; i < 1024; ++i)
		remap_dir[i] = 0;
	for(i = 0; i < 256; ++i)
	{
		entry = (i * PAGE_SIZE * 1024) | flags;
		remap_dir[i] = entry;
		remap_dir[256 * 3 + i] = entry;
	}
	pse_enable(&remap_dir);
	gdt_move();
}
