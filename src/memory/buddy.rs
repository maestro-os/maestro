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

use crate::memory::NULL;
use crate::memory::Void;
use crate::memory;
use crate::util;

/*
 * Type representing the order of a memory frame.
 */
pub type FrameOrder = u8;
/*
 * Type representing buddy allocator flags.
 */
pub type Flags = i32;

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

/*
 * Structure representing an allocatable zone of memory.
 */
struct Zone {
	/* TODO doc */
	type_: Flags,
	/* TODO doc */
	spinlock: util::Spinlock, // TODO Use a semaphore
	/* TODO doc */
	allocated_pages: usize,

	/* TODO doc */
	begin: *mut Void,
	/* TODO doc */
	size: usize,
}

impl Zone {
	/*
	 * Creates a new instance of zone with type `type_`. The zone covers the memory from pointer `begin` to
	 * `begin + size` where `size` if the size in bytes.
	 */
	pub fn new(type_: Flags, begin: *mut Void, size: usize) -> Self {
		Self {
			type_: type_,
			spinlock: util::Spinlock::new(),
			allocated_pages: 0,

			begin: begin,
			size: size,
		}
	}

	/*
	 * Creates a fake Zone. This function is only meant to fill the global variable array until it gets really filled
	 * by the initialization function.
	 */
	pub const fn fake() -> Self {
		Self {
			type_: 0,
			spinlock: util::Spinlock::new(),
			allocated_pages: 0,
			begin: 0 as _,
			size: 0,
		}
	}

	/*
	 * Returns the number of allocated pages in the current zone of memory.
	 */
	pub fn get_allocated_pages(&self) -> usize {
		self.allocated_pages
	}

	// TODO
}

// TODO OOM killer

static mut ZONES: [Zone; 3] = [
	Zone::fake(),
	Zone::fake(),
	Zone::fake(),
];

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
	order
}

/*
 * Initializes the buddy allocator.
 */
pub fn init() {
	// TODO Initialize zones
}

/*
 * Allocates a frame of memory using the buddy allocator.
 */
pub fn alloc(_order: FrameOrder, _flags: Flags) -> *mut Void {
	// TODO
	memory::NULL as _
}

/*
 * Uses `alloc` and zeroes the allocated frame.
 */
pub fn alloc_zero(order: FrameOrder, flags: Flags) -> *mut Void {
	let ptr = alloc(order, flags);

	if ptr != (NULL as _) {
		let len = get_frame_size(order);
		unsafe {
			util::bzero(ptr, len);
		}
	}
	ptr
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
	let mut n = 0;

	unsafe {
		for i in 0..ZONES.len() {
			n += ZONES[i].get_allocated_pages();
		}
	}
	n
}
