/*
 * This module contains the buddy allocator which allows to allocate 2^^n pages
 * large frames of memory.
 *
 * This allocator works by dividing frames of memory in two until the a frame of
 * the required size is available.
 *
 * The order of a frame is the `n` in the expression `2^^n` that represents the
 * size of a frame in pages.
 */

use crate::memory::Void;
use crate::memory;
use crate::util;

/*
 * Type representing the order of a memory frame.
 */
type FrameOrder = u8;
/*
 * Type representing buddy allocator flags.
 */
type Flags = i32;

/*
 * The maximum order of a buddy allocated frame.
 */
pub const MAX_ORDER: FrameOrder = 17;

/*
 * Buddy allocator flag. Tells that the allocated frame must be mapped into the user zone.
 */
pub const FLAG_ZONE_USER: Flags = 0b000;
/*
 * Buddy allocator flag. Tells that the allocated frame must be mapped into the kernel zone.
 */
pub const FLAG_ZONE_KERNEL: Flags = 0b001;
/*
 * Buddy allocator flag. Tells that the allocated frame must be mapped into the DMA zone.
 */
pub const FLAG_ZONE_DMA: Flags = 0b010;
/*
 * Buddy allocator flag. Tells that the allocation shall not fail (unless not enough memory is present on the system).
 * This flag is ignored if FLAG_USER is not specified or if the allocation order is higher than 0.
 * The allocator shall use the OOM killer to recover memory.
 */
pub const FLAG_NOFAIL: Flags = 0b100;

// TODO OOM killer

/*
 * The spinlock used for buddy allocator operations.
 */
static SPINLOCK: util::Spinlock = util::Spinlock::new();

/*
 * The size in bytes of a frame allocated by the buddy allocator with the given `order`.
 */
pub fn get_frame_size(order: FrameOrder) -> usize {
	memory::PAGE_SIZE << order
}

/*
 * Returns the buddy order required to fit the given number of pages.
 */
pub fn get_order(pages: usize) -> FrameOrder {
	let mut order: FrameOrder = 0;
	let mut i = 1;

	while i < pages {
		i *= 2;
		order += 1;
	}
	return order;

}

/*
 * Initializes the buddy allocator.
 */
pub fn init() {
	// TODO
}

/*
 * Allocates a frame of memory using the buddy allocator.
 */
pub fn alloc(_order: FrameOrder, _flags: Flags) -> *const Void {
	// TODO
	memory::NULL
}

/*
 * Uses `alloc` and zeroes the allocated frame.
 */
pub fn alloc_zero(_order: FrameOrder, _flags: Flags) -> *const Void {
	// TODO
	memory::NULL
}

/*
 * Frees the given memory frame that was allocated using the buddy allocator. The given order must be the same as the
 * one given to allocate the frame.
 */
pub fn free(_ptr: *const Void, _order: FrameOrder) {
	// TODO
}

/*
 * Returns the total number of pages allocated by the buddy allocator.
 */
pub fn allocated_pages() -> usize {
	// TODO
	0
}
