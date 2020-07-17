#ifndef MEM_SPACE_H
# define MEM_SPACE_H

# include <util/util.h>

/*
 * The beginning of the virtual memory for a memory space.
 */
# define MEM_SPACE_BEGIN	((void *) 0x1000)
/*
 * The end of the virtual memory for a memory space.
 */
# define MEM_SPACE_END		((void *) ~(PAGE_SIZE - 1))

/*
 * Memory region flag allowing write permission on the region.
 */
# define MEM_REGION_FLAG_WRITE		0b000001
/*
 * Memory region flag allowing execution permission on the region.
 */
# define MEM_REGION_FLAG_EXEC		0b000010
/*
 * Memory region flag telling that the region is shared with other memory
 * spaces.
 */
# define MEM_REGION_FLAG_SHARED		0b000100
/*
 * Memory region flag telling that the region is a stack.
 */
# define MEM_REGION_FLAG_STACK		0b001000
/*
 * Memory region flag telling that the region is a userspace region.
 */
# define MEM_REGION_FLAG_USER		0b010000
/*
 * Memory region flag telling that the region must have the same virtual and
 * physical address.
 */
# define MEM_REGION_FLAG_IDENTITY	0b100000

#define ASSERT_RANGE(addr, pages)\
	debug_assert((uintptr_t) (addr)\
		< (uintptr_t) (addr) + ((pages) * PAGE_SIZE), "mem_space: invalid gap")

typedef struct mem_space mem_space_t;

/*
 * Structure representing a memory region in the memory space. (Used addresses)
 */
typedef struct mem_region
{
	/* Linked list of memory regions in the current memory space. */
	list_head_t list;
	/*
	 * Double-linked list of memory regions that share the same physical space.
	 * Elements in this list might not be in the same memory space.
	 */
	list_head_t shared_list;
	/* The node of the tree the structure is stored in. */
	avl_tree_t node;
	/* The memory space associated with the region. */
	mem_space_t *mem_space;

	/* The flags for the memory region. */
	char flags;
	/* The beginning address of the region. */
	void *begin;
	/* The size of the region in pages. */
	size_t pages;
	/* The number of used pages in the region. */
	size_t used_pages;
} mem_region_t;

/*
 * Structure representing a memory gap int the memory space. (Free addresses)
 */
typedef struct mem_gap
{
	/* Double-linked list of memory gaps in the current memory space. */
	list_head_t list;
	/* The node of the tree the structure is stored in */
	avl_tree_t node;
	/* The memory space associated with the gap. */
	mem_space_t *mem_space;

	/* The beginning address of the gap. */
	void *begin;
	/* The size of the gap in pages. */
	size_t pages;
} mem_gap_t;

/*
 * Structure representing a memory context. Allowing to allocate virtual memory.
 */
struct mem_space
{
	/* Linked list of regions (used zones) */
	list_head_t *regions;
	/* Linked list of gaps (free zones, ordered by growing pointer) */
	list_head_t *gaps;
	/* Binary tree of regions (ordered by pointer) */
	avl_tree_t *used_tree;
	/* Binary tree of gaps (ordered by size in pages) */
	avl_tree_t *free_tree;

	/* The spinlock for this memory space. */
	spinlock_t spinlock;

	/* An architecture dependent object to handle memory permissions. */
	void *page_dir;
};

mem_space_t *mem_space_init(void);
mem_space_t *mem_space_clone(mem_space_t *space);
void *mem_space_alloc(mem_space_t *space, size_t pages, int flags);
void *mem_space_alloc_fixed(mem_space_t *space, void *addr, size_t pages,
	int flags);
void *mem_space_alloc_kernel_stack(mem_space_t *space, size_t buddy_order);
int mem_space_free(mem_space_t *space, void *ptr, size_t pages);
int mem_space_free_stack(mem_space_t *space, void *stack);
int mem_space_can_access(mem_space_t *space, const void *ptr, size_t size,
	int write);
void mem_space_copy_from(mem_space_t *space, void *dst, const void *src,
	size_t n);
void mem_space_copy_to(mem_space_t *space, void *dst, const void *src,
	size_t n);
int mem_space_handle_page_fault(mem_space_t *space, void *ptr, int error_code);
void mem_space_destroy(mem_space_t *space);

mem_region_t *region_create(mem_space_t *space, char flags, void *begin,
	size_t pages, size_t used_pages);
mem_region_t *region_clone(mem_space_t *space, mem_region_t *r);
int region_is_shared(mem_region_t *region);
void region_free(mem_region_t *region);
void regions_free(list_head_t *list);
int regions_clone(mem_space_t *dest, list_head_t *regions);
void regions_disable_write(mem_region_t *r);
mem_region_t *region_find(avl_tree_t *n, void *ptr);
int region_split(mem_region_t *r, void *addr, size_t pages);
void regions_update_near(mem_region_t *region);
void region_copy_pages(mem_region_t *dest, mem_region_t *src);

int region_phys_default(mem_region_t *r);
void region_phys_identity(mem_region_t *r);
int region_phys_alloc(mem_region_t *r);
void region_phys_free(mem_region_t *r);

mem_gap_t *gap_create(mem_space_t *space, void *begin, size_t pages);
mem_gap_t *gap_clone(mem_space_t *dest, mem_gap_t *g);
int gaps_clone(mem_space_t *dest, list_head_t *gaps);
avl_tree_t *gap_find(avl_tree_t *n, size_t pages);
int gaps_init(mem_space_t *s);
int gap_extend(avl_tree_t **tree, void *addr, size_t pages);
void gap_shrink(avl_tree_t **tree, avl_tree_t *gap, size_t pages);
void gap_free(mem_gap_t *gap);
void gaps_free(list_head_t *list);

#endif
