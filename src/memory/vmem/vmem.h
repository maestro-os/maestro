#ifndef VMEM_H
# define VMEM_H

# include <stdint.h>
# include <libc/string.h>

/*
 * The object used in x86 memory permissions handling.
 */
typedef uint32_t *vmem_t;

extern vmem_t kernel_vmem;

vmem_t vmem_init(void);
void vmem_kernel(void);
uint32_t *vmem_resolve(vmem_t vmem, const void *ptr);
int vmem_is_mapped(vmem_t vmem, const void *ptr);
int vmem_contains(vmem_t vmem, const void *ptr, size_t size);
void *vmem_translate(vmem_t vmem, const void *ptr);
uint32_t vmem_get_entry(vmem_t vmem, const void *ptr);
void vmem_map(vmem_t vmem, const void *physaddr, const void *virtaddr,
	int flags);
void vmem_map_pse(vmem_t vmem, const void *physaddr, const void *virtaddr,
	int flags);
void vmem_map_range(vmem_t vmem, const void *physaddr, const void *virtaddr,
	size_t pages, int flags);
void vmem_identity(vmem_t vmem, const void *page, int flags);
void vmem_identity_pse(vmem_t vmem, const void *page, int flags);
void vmem_identity_range(vmem_t vmem, const void *from, size_t pages,
	int flags);
void vmem_unmap(vmem_t vmem, const void *virtaddr);
void vmem_unmap_range(vmem_t vmem, const void *virtaddr, size_t pages);
vmem_t vmem_clone(vmem_t vmem);
void vmem_flush(vmem_t vmem);
void vmem_destroy(vmem_t vmem);

extern uint32_t cr0_get(void);
extern void cr0_set(uint32_t flags);
extern void cr0_clear(uint32_t flags);
extern void *cr2_get(void);
extern void *cr3_get(void);

extern void paging_enable(vmem_t vmem);
extern void paging_disable(void);
extern void tlb_reload(void);

#endif
