#include <memory/memory.h>
#include <util/util.h>

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
ATTR_ALIGNED(PAGE_SIZE)
ATTR_SECTION(".boot.data")
static uint32_t remap_dir[1024];

extern void pse_enable(void *page_dir);
extern void kernel_remap_update_stack(void);

/*
 * Remaps the kernel image to higher half memory. This function enables PSE.
 * Note: the kernel can access the NULL pointer and write onto its own code
 * after this function. Thus the kernel must be protected as soon as possible.
 */
ATTR_SECTION(".boot.text")
void kernel_remap(void)
{
	const uint32_t flags = PAGING_TABLE_PAGE_SIZE | PAGING_TABLE_WRITE
		| PAGING_TABLE_PRESENT;

	remap_dir[0] = flags;
	remap_dir[768] = flags;
	pse_enable(&remap_dir);
}
