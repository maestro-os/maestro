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

use core::mem::MaybeUninit;
use crate::memory::NULL;
use crate::memory::Void;
use crate::memory;
use crate::util::lock::Mutex;
use crate::util::lock::MutexGuard;
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
 * Buddy allocator flag. Tells that the allocation shall not fail (unless not enough memory is
 * present on the system). This flag is ignored if FLAG_USER is not specified or if the allocation
 * order is higher than 0. The allocator shall use the OOM killer to recover memory.
 */
pub const FLAG_NOFAIL: Flags = 0b100;

// TODO OOM killer


/*
 * Structure representing an allocatable zone of memory.
 */
struct Zone {
	/* TODO doc */
	type_: Flags,
	/* TODO doc */
	allocated_pages: usize,

	/* TODO doc */
	begin: *mut Void,
	/* TODO doc */
	size: usize,
}

// TODO Remplace by a linked list? (in case of holes in memory)
/*
 * The array of buddy allocator zones.
 */
static mut ZONES: MaybeUninit<[Mutex<Zone>; 3]> = MaybeUninit::uninit();

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
	unsafe {
		util::zero_object(&mut ZONES);

		// TODO Init zones according to memory mapping
		let z = ZONES.get_mut();

		z[0].lock().get_mut().init(FLAG_ZONE_USER, 0 as *mut _, 0); // TODO
		z[0].unlock();

		z[1].lock().get_mut().init(FLAG_ZONE_KERNEL, 0 as *mut _, 0); // TODO
		z[1].unlock();

		// TODO
		z[2].lock().get_mut().init(FLAG_ZONE_DMA, 0 as *mut _, 0); // TODO
		z[2].unlock();
	}
}

/*
 * Allocates a frame of memory using the buddy allocator.
 */
pub fn alloc(order: FrameOrder, _flags: Flags) -> *mut Void {
	debug_assert!(order <= MAX_ORDER);

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
 * Frees the given memory frame that was allocated using the buddy allocator. The given order must
 * be the same as the one given to allocate the frame.
 */
pub fn free(_ptr: *const Void, order: FrameOrder) {
	debug_assert!(order <= MAX_ORDER);

	// TODO
}

/*
 * Returns the total number of pages allocated by the buddy allocator.
 */
pub fn allocated_pages() -> usize {
	let mut n = 0;

	unsafe {
		let z = ZONES.get_mut();
		for i in 0..z.len() {
			let guard = MutexGuard::new(&mut z[i]); // TODO Remove `mut`?
			n += guard.get().get_allocated_pages();
		}
	}
	n
}

impl Zone {
	/*
	 * Initializes the zone with type `type_`. The zone covers the memory from pointer `begin` to
	 * `begin + size` where `size` is the size in bytes.
	 */
	pub fn init(&mut self, type_: Flags, begin: *mut Void, size: usize) {
		self.type_ = type_;
		self.allocated_pages = 0;
		self.begin = begin;
		self.size = size;
	}

	/*
	 * Returns the number of allocated pages in the current zone of memory.
	 */
	pub fn get_allocated_pages(&self) -> usize {
		self.allocated_pages
	}

	// TODO
}
