/*
 * Copyright 2024 Luc Lenôtre
 *
 * This file is part of Maestro.
 *
 * Maestro is free software: you can redistribute it and/or modify it under the
 * terms of the GNU General Public License as published by the Free Software
 * Foundation, either version 3 of the License, or (at your option) any later
 * version.
 *
 * Maestro is distributed in the hope that it will be useful, but WITHOUT ANY
 * WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR
 * A PARTICULAR PURPOSE. See the GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along with
 * Maestro. If not, see <https://www.gnu.org/licenses/>.
 */

//! The buddy allocator allows to allocate blocks of `2^^n` pages of memory.
//!
//! This allocator works by dividing frames of memory in two recursively until a frame
//! of the required size is available.
//!
//! The order of a frame is the `n` in the expression `pow(2, n)` that represents the
//! size of a frame in pages.

use super::stats;
use crate::memory;
use core::{
	alloc::AllocError,
	cmp::min,
	ffi::c_void,
	intrinsics::likely,
	mem::size_of,
	ptr::{null_mut, NonNull},
	slice,
};
use macros::instrument_allocator;
use utils::{errno::AllocResult, lock::IntMutex, math};

/// The order of a memory frame.
pub type FrameOrder = u8;
/// Buddy allocator flags.
pub type Flags = i32;
/// The identifier of a frame.
type FrameID = u32;

/// The maximum order of a buddy allocated frame.
pub const MAX_ORDER: FrameOrder = 17;

/// The number of memory zones.
pub const ZONES_COUNT: usize = 3;

/// The mask for the zone ID in buddy allocator flags.
const ZONE_TYPE_MASK: Flags = 0b11;

/// Buddy allocator flag: allocate in user zone
pub const FLAG_ZONE_TYPE_USER: Flags = 0b00;
/// Buddy allocator flag: allocate in MMIO zone
pub const FLAG_ZONE_TYPE_MMIO: Flags = 0b01;
/// Buddy allocator flag: allocate in kernel zone
pub const FLAG_ZONE_TYPE_KERNEL: Flags = 0b10;

/// Value indicating that the frame is used.
pub const FRAME_STATE_USED: FrameID = !0_u32;

/// An allocatable zone of memory, initialized at boot.
pub(crate) struct Zone {
	/// A pointer to the beginning of the metadata of the zone
	metadata_begin: *mut Frame,
	/// A pointer to the beginning of the allocatable memory of the zone
	begin: *mut c_void,
	/// The size of the zone in pages
	pages_count: FrameID,
	/// The number of allocated pages in the zone
	allocated_pages: usize,
	/// The free list containing linked lists to free frames. Each linked list contain frames of
	/// the order corresponding to the element in this array
	free_list: [Option<NonNull<Frame>>; (MAX_ORDER + 1) as usize],
}

impl Zone {
	/// Returns a value for use as a placeholder until boot-time initialization has been performed.
	const fn placeholder() -> Self {
		Self {
			metadata_begin: null_mut(),
			begin: null_mut(),
			pages_count: 0,
			allocated_pages: 0,
			free_list: [None; (MAX_ORDER + 1) as usize],
		}
	}
}

impl Zone {
	/// Fills the free list during initialization according to the number of
	/// available pages.
	fn fill_free_list(&mut self) {
		let frames = self.frames();
		let mut frame: FrameID = 0;
		let mut order = MAX_ORDER;
		while frame < self.pages_count as FrameID {
			// Check the order fits in remaining pages
			let p = math::pow2(order as FrameID) as FrameID;
			if frame + p > self.pages_count {
				order -= 1;
				continue;
			}
			// Init frame
			let f = &mut frames[frame as usize];
			f.mark_free(self);
			f.order = order;
			f.link(self);
			// Jump to next offset
			frame += p;
		}
		#[cfg(config_debug_debug)]
		self.check_free_list();
	}

	/// Creates a buddy allocator zone.
	///
	/// The zone covers the memory from pointer `begin` to `begin + size` where `size` is the size
	/// in bytes.
	///
	/// `metadata_begin` must be a virtual address and `begin` must be a
	/// physical address.
	pub(crate) fn new(
		metadata_begin: *mut c_void,
		pages_count: FrameID,
		begin: *mut c_void,
	) -> Zone {
		let mut z = Zone {
			metadata_begin: metadata_begin as _,
			begin,
			pages_count,
			allocated_pages: 0,
			free_list: Default::default(),
		};
		z.fill_free_list();
		z
	}

	/// Returns the size in bytes of the allocatable memory.
	#[inline]
	fn get_size(&self) -> usize {
		(self.pages_count as usize) * memory::PAGE_SIZE
	}

	/// Returns an available frame owned by this zone, with an order of at least
	/// `order`.
	fn get_available_frame(&mut self, order: FrameOrder) -> Option<NonNull<Frame>> {
		let mut frame = self.free_list[(order as usize)..]
			.iter_mut()
			.filter_map(|f| *f)
			.next()?;
		let f = unsafe { frame.as_mut() };
		debug_assert!(!f.is_used());
		debug_assert!((f.memory_ptr(self) as usize) >= (self.begin as usize));
		debug_assert!((f.memory_ptr(self) as usize) < (self.begin as usize) + self.get_size());
		Some(frame)
	}

	/// Returns the identifier for the frame at the given pointer `ptr`.
	///
	/// The pointer must point to the frame itself, not the Frame structure.
	fn get_frame_id_from_ptr(&self, ptr: *const c_void) -> FrameID {
		(((ptr as usize) - (self.begin as usize)) / memory::PAGE_SIZE) as _
	}

	/// Returns a mutable slice over the metadata of the zone's frames.
	#[inline]
	fn frames(&self) -> &'static mut [Frame] {
		unsafe { slice::from_raw_parts_mut(self.metadata_begin, self.pages_count as usize) }
	}

	/// Checks the correctness of the free list for the zone.
	///
	/// Every frame in the free list must have an order equal to the order of the bucket it's
	/// inserted in and must be free.
	///
	/// If a frame is the first of a list, it must not have a previous element.
	///
	/// If a frame is invalid, the function shall result in the kernel
	/// panicking.
	#[cfg(config_debug_debug)]
	fn check_free_list(&self) {
		let zone_size = (self.pages_count as usize) * memory::PAGE_SIZE;
		let frames = self.frames();
		for (order, list) in self.free_list.iter().enumerate() {
			let Some(mut first) = *list else {
				continue;
			};
			let mut frame = unsafe { first.as_mut() };
			let mut is_first = true;
			// Iterate on linked list
			loop {
				let id = frame.get_id(self);
				#[cfg(config_debug_debug)]
				frame.check_broken(self);
				debug_assert!(!frame.is_used());
				debug_assert_eq!(frame.order, order as _);
				debug_assert!(!is_first || frame.prev == id);

				let frame_ptr = frame.memory_ptr(self);
				debug_assert!(frame_ptr >= self.begin);
				unsafe {
					let zone_end = self.begin.add(zone_size);
					debug_assert!(frame_ptr < zone_end);
					debug_assert!(frame_ptr.add(frame.get_size()) <= zone_end);
				}

				if frame.next == id {
					break;
				}
				frame = &mut frames[frame.next as usize];
				is_first = false;
			}
		}
	}
}

/// The metadata for a frame of physical memory.
///
/// The structure has an internal linked list for the free list.
/// This linked list doesn't store pointers but frame identifiers to save memory.
///
/// If either `prev` or `next` has value [`FRAME_STATE_USED`], the frame is marked as used.
///
/// If a frame points to itself, it means that no more elements are present in
/// the list.
#[repr(packed)]
struct Frame {
	/// Identifier of the previous frame in the free list.
	prev: FrameID,
	/// Identifier of the next frame in the free list.
	next: FrameID,

	/// Order of the current frame
	order: FrameOrder,
}

impl Frame {
	/// Returns the id of the current frame in the associated zone `zone`.
	fn get_id(&self, zone: &Zone) -> FrameID {
		let self_off = self as *const _ as usize;
		let zone_off = zone.metadata_begin as *const _ as usize;
		debug_assert!(self_off >= zone_off);

		((self_off - zone_off) / size_of::<Self>()) as u32
	}

	/// Returns the identifier of the buddy frame in zone `zone`, taking in
	/// account the frame's order.
	///
	/// The caller has the responsibility to check that it is below the number of frames in the
	/// zone.
	#[inline]
	fn get_buddy_id(&self, zone: &Zone) -> FrameID {
		self.get_id(zone) ^ (1 << self.order) as u32
	}

	/// Returns the pointer to the location of the associated physical memory.
	fn memory_ptr(&self, zone: &Zone) -> *mut c_void {
		let off = self.get_id(zone) as usize * memory::PAGE_SIZE;
		(zone.begin as usize + off) as _
	}

	/// Tells whether the frame is used or not.
	#[inline]
	fn is_used(&self) -> bool {
		(self.prev == FRAME_STATE_USED) || (self.next == FRAME_STATE_USED)
	}

	/// Returns the size of the frame in bytes.
	#[inline]
	fn get_size(&self) -> usize {
		get_frame_size(self.order)
	}

	/// Marks the frame as used. The frame must not be linked to any free list.
	#[inline]
	fn mark_used(&mut self) {
		self.prev = FRAME_STATE_USED;
		self.next = FRAME_STATE_USED;
	}

	/// Marks the frame as free. The frame must not be linked to any free list.
	#[inline]
	fn mark_free(&mut self, zone: &Zone) {
		let id = self.get_id(zone);
		self.prev = id;
		self.next = id;
	}

	/// Debug function to assert that the chunk is valid.
	///
	/// Invalid chunk shall result in the kernel panicking.
	#[cfg(config_debug_debug)]
	fn check_broken(&self, zone: &Zone) {
		debug_assert!(self.prev == FRAME_STATE_USED || self.prev < zone.pages_count);
		debug_assert!(self.next == FRAME_STATE_USED || self.next < zone.pages_count);
		debug_assert!(self.order <= MAX_ORDER);
	}

	/// Links the frame into zone `zone`'s free list.
	fn link(&mut self, zone: &mut Zone) {
		#[cfg(config_debug_debug)]
		self.check_broken(zone);
		#[cfg(config_debug_debug)]
		zone.check_free_list();
		debug_assert!(!self.is_used());

		let id = self.get_id(zone);
		self.prev = id;
		self.next = if let Some(mut next) = zone.free_list[self.order as usize] {
			let next = unsafe { next.as_mut() };
			debug_assert!(!next.is_used());
			next.prev = id;
			next.get_id(zone)
		} else {
			id
		};
		zone.free_list[self.order as usize] = NonNull::new(self);

		#[cfg(config_debug_debug)]
		self.check_broken(zone);
		#[cfg(config_debug_debug)]
		zone.check_free_list();
	}

	/// Unlinks the frame from zone `zone`'s free list. The frame must not be
	/// used.
	fn unlink(&mut self, zone: &mut Zone) {
		#[cfg(config_debug_debug)]
		self.check_broken(zone);
		debug_assert!(!self.is_used());
		#[cfg(config_debug_debug)]
		zone.check_free_list();

		let frames = zone.frames();
		let id = self.get_id(zone);
		let has_prev = self.prev != id;
		let has_next = self.next != id;

		let first = &mut zone.free_list[self.order as usize];
		if first.map(NonNull::as_ptr) == Some(self) {
			*first = if has_next {
				NonNull::new(&mut frames[self.next as usize])
			} else {
				None
			};
		}

		if has_prev {
			frames[self.prev as usize].next = if has_next { self.next } else { self.prev };
		}
		if has_next {
			frames[self.next as usize].prev = if has_prev { self.prev } else { self.next };
		}

		#[cfg(config_debug_debug)]
		self.check_broken(zone);
		#[cfg(config_debug_debug)]
		zone.check_free_list();
	}

	/// Unlinks the frame from zone `zone`'s free list, splits it until it
	/// reaches the required order `order` while linking the new free frames to
	/// the free list.
	///
	/// At the end of the function, the current frame is **not** linked to the free list.
	///
	/// The frame must not be marked as used.
	fn split(&mut self, zone: &mut Zone, order: FrameOrder) {
		#[cfg(config_debug_debug)]
		self.check_broken(zone);
		debug_assert!(!self.is_used());
		debug_assert!(order <= MAX_ORDER);
		debug_assert!(self.order >= order);

		let frames = zone.frames();

		self.unlink(zone);
		while self.order > order {
			self.order -= 1;
			// Get buddy ID
			let buddy = self.get_buddy_id(zone);
			if buddy >= zone.pages_count {
				break;
			}
			// Update buddy
			let buddy_frame = &mut frames[buddy as usize];
			buddy_frame.mark_free(zone);
			buddy_frame.order = self.order;
			buddy_frame.link(zone);
		}

		#[cfg(config_debug_debug)]
		self.check_broken(zone);
	}

	/// Coalesces the frame in zone `zone` with free buddy blocks recursively
	/// until no buddy is available anymore.
	///
	/// The current frame must not be marked as used.
	///
	/// Buddies that are merged with the frame are unlinked.
	///
	/// The order of the frame is incremented at each merge.
	///
	/// The frame is linked to the free list by the function.
	fn coalesce(&mut self, zone: &mut Zone) {
		#[cfg(config_debug_debug)]
		self.check_broken(zone);
		debug_assert!(!self.is_used());

		let frames = zone.frames();

		while self.order < MAX_ORDER {
			let id = self.get_id(zone);
			// Get buddy ID
			let buddy = self.get_buddy_id(zone);
			if buddy >= zone.pages_count {
				break;
			}
			// Check if coalesce is possible
			let new_pages_count = math::pow2((self.order + 1) as usize) as FrameID;
			if min(id, buddy) + new_pages_count > zone.pages_count {
				break;
			}
			let buddy_frame = &mut frames[buddy as usize];
			#[cfg(config_debug_debug)]
			buddy_frame.check_broken(zone);
			if buddy_frame.order != self.order || buddy_frame.is_used() {
				break;
			}
			// Update buddy
			buddy_frame.unlink(zone);
			if id < buddy {
				self.order += 1;
			} else {
				buddy_frame.order += 1;
				buddy_frame.coalesce(zone);
				return;
			}
		}

		#[cfg(config_debug_debug)]
		zone.check_free_list();
		self.link(zone);
		#[cfg(config_debug_debug)]
		self.check_broken(zone);
	}
}

/// The array of buddy allocator zones.
pub(crate) static ZONES: IntMutex<[Zone; ZONES_COUNT]> = IntMutex::new([
	Zone::placeholder(),
	Zone::placeholder(),
	Zone::placeholder(),
]);

/// The size in bytes of a frame with the given order `order`.
#[inline]
pub fn get_frame_size(order: FrameOrder) -> usize {
	memory::PAGE_SIZE << order
}

/// Returns the buddy order required to fit the given number of pages.
#[inline]
pub fn get_order(pages: usize) -> FrameOrder {
	if likely(pages != 0) {
		(u32::BITS - pages.leading_zeros()) as _
	} else {
		0
	}
}

/// Returns the size of the metadata for one frame.
#[inline]
pub const fn get_frame_metadata_size() -> usize {
	size_of::<Frame>()
}

/// Returns a mutable reference to the zone that contains the given pointer `ptr`.
///
/// `zones` is the list of zones.
fn get_zone_for_pointer(zones: &mut [Zone; ZONES_COUNT], ptr: *const c_void) -> Option<&mut Zone> {
	zones
		.iter_mut()
		.find(|z| ptr >= z.begin && (ptr as usize) < (z.begin as usize) + z.get_size())
}

/// Allocates a frame of memory using the buddy allocator.
///
/// Arguments:
/// - `order` is the order of the frame to be allocated
/// - `flags` for the allocation
///
/// If no suitable frame is found, the function returns an error.
///
/// On success, the function returns a *physical* pointer to the allocated memory.
#[instrument_allocator(name = buddy, op = alloc, size = order, scale = log2)]
pub fn alloc(order: FrameOrder, flags: Flags) -> AllocResult<NonNull<c_void>> {
	if order > MAX_ORDER {
		return Err(AllocError);
	}
	// Select a zone and frame to allocate on
	let mut zones = ZONES.lock();
	let begin_zone = (flags & ZONE_TYPE_MASK) as usize;
	let (mut frame, zone) = zones[begin_zone..]
		.iter_mut()
		.filter_map(|z| Some((z.get_available_frame(order)?, z)))
		.next()
		.ok_or(AllocError)?;
	let frame = unsafe { frame.as_mut() };
	// Do the actual allocation
	debug_assert!(!frame.is_used());
	frame.split(zone, order);
	let ptr = frame.memory_ptr(zone);
	debug_assert!(ptr.is_aligned_to(memory::PAGE_SIZE));
	debug_assert!(ptr >= zone.begin && ptr < (zone.begin as usize + zone.get_size()) as _);
	frame.mark_used();
	// Statistics
	let pages_count = math::pow2(order as usize);
	zone.allocated_pages += pages_count;
	stats::MEM_INFO.lock().mem_free -= pages_count * 4;
	NonNull::new(ptr).ok_or(AllocError)
}

/// Calls [`alloc()`] with order `order`, allocating in the kernel zone.
///
/// The function returns the *virtual* address, not the physical one.
pub fn alloc_kernel(order: FrameOrder) -> AllocResult<NonNull<c_void>> {
	let ptr = alloc(order, FLAG_ZONE_TYPE_KERNEL)?;
	let virt_ptr = memory::kern_to_virt(ptr.as_ptr()) as _;
	NonNull::new(virt_ptr).ok_or(AllocError)
}

/// Frees the given memory frame that was allocated using the buddy allocator.
///
/// Arguments:
/// - `ptr` is the *virtual* address to the beginning of the frame
/// - `order` is the order of the frame
///
/// The given order must be the same as the one given to [`alloc()`].
///
/// # Safety
///
/// If the `ptr` or `order` are invalid, the behaviour is undefined.
///
/// Using the memory referenced by the pointer after freeing results in an undefined behaviour.
#[instrument_allocator(name = buddy, op = free, ptr = ptr, size = order, scale = log2)]
pub unsafe fn free(ptr: *const c_void, order: FrameOrder) {
	debug_assert!(ptr.is_aligned_to(memory::PAGE_SIZE));
	debug_assert!(order <= MAX_ORDER);
	// Get zone
	let mut zones = ZONES.lock();
	let zone = get_zone_for_pointer(&mut zones, ptr).unwrap();
	let frames = zone.frames();
	// Perform free
	let frame_id = zone.get_frame_id_from_ptr(ptr);
	debug_assert!(frame_id < zone.pages_count);
	let frame = &mut frames[frame_id as usize];
	debug_assert!(frame.is_used());
	frame.mark_free(zone);
	frame.coalesce(zone);
	// Statistics
	let pages_count = math::pow2(order as usize);
	zone.allocated_pages -= pages_count;
	stats::MEM_INFO.lock().mem_free += pages_count * 4;
}

/// Frees the given memory frame.
///
/// Arguments:
/// - `ptr` is the *virtual* address to the beginning of the frame
/// - `order` is the order of the frame
///
/// # Safety
///
/// See [`free`]
pub unsafe fn free_kernel(ptr: *const c_void, order: FrameOrder) {
	free(memory::kern_to_phys(ptr), order);
}

/// Returns the total number of pages allocated by the buddy allocator.
pub fn allocated_pages_count() -> usize {
	let zones = ZONES.lock();
	zones.iter().map(|z| z.allocated_pages).sum()
}

#[cfg(test)]
mod test {
	use super::*;
	use core::{ptr::null, slice};

	#[test_case]
	fn buddy0() {
		let alloc_pages = allocated_pages_count();
		unsafe {
			let p = alloc_kernel(0).unwrap();
			let slice = slice::from_raw_parts_mut(p.as_ptr() as *mut u8, get_frame_size(0));
			slice.fill(!0);
			free_kernel(p.as_ptr(), 0);
		}
		debug_assert_eq!(allocated_pages_count(), alloc_pages);
	}

	#[test_case]
	fn buddy1() {
		let alloc_pages = allocated_pages_count();
		unsafe {
			let p = alloc_kernel(1).unwrap();
			let slice = slice::from_raw_parts_mut(p.as_ptr() as *mut u8, get_frame_size(0));
			slice.fill(!0);
			free_kernel(p.as_ptr(), 1);
		}
		debug_assert_eq!(allocated_pages_count(), alloc_pages);
	}

	fn lifo_test(i: usize) {
		unsafe {
			let p = alloc_kernel(0).unwrap();
			let slice = slice::from_raw_parts_mut(p.as_ptr() as *mut u8, get_frame_size(0));
			slice.fill(!0);
			if i > 0 {
				lifo_test(i - 1);
			}
			free_kernel(p.as_ptr(), 0);
		}
	}

	#[test_case]
	fn buddy_lifo() {
		let alloc_pages = allocated_pages_count();
		lifo_test(100);
		debug_assert_eq!(allocated_pages_count(), alloc_pages);
	}

	#[test_case]
	fn buddy_fifo() {
		let alloc_pages = allocated_pages_count();
		unsafe {
			let mut frames: [*const c_void; 100] = [null::<c_void>(); 100];
			for frame in &mut frames {
				let p = alloc_kernel(0).unwrap();
				*frame = p.as_ptr();
			}
			for frame in frames {
				free_kernel(frame, 0);
			}
		}
		debug_assert_eq!(allocated_pages_count(), alloc_pages);
	}

	fn get_dangling(order: FrameOrder) -> *mut c_void {
		unsafe {
			let p = alloc_kernel(order).unwrap();
			let slice = slice::from_raw_parts_mut(p.as_ptr() as *mut u8, get_frame_size(0));
			slice.fill(!0);
			free_kernel(p.as_ptr(), 0);
			p.as_ptr()
		}
	}

	#[test_case]
	fn buddy_free() {
		let alloc_pages = allocated_pages_count();
		let first = get_dangling(0);
		for _ in 0..100 {
			assert_eq!(get_dangling(0), first);
		}
		debug_assert_eq!(allocated_pages_count(), alloc_pages);
	}

	struct TestDupNode {
		next: Option<NonNull<TestDupNode>>,
	}

	unsafe fn has_cycle(mut begin: NonNull<TestDupNode>) -> bool {
		let mut tortoise = Some(begin);
		let mut hoare = begin.as_mut().next;
		while let (Some(mut t), Some(mut h)) = (tortoise, hoare) {
			if t.as_ptr() == h.as_ptr() {
				return true;
			}
			tortoise = t.as_mut().next;
			hoare = h.as_mut().next.and_then(|mut h| h.as_mut().next);
		}
		false
	}

	/// Testing whether the allocator returns pages that are already allocated
	#[test_case]
	fn buddy_full_duplicate() {
		let alloc_pages = allocated_pages_count();
		unsafe {
			let mut first: Option<NonNull<TestDupNode>> = None;
			while let Ok(p) = alloc_kernel(0) {
				let mut node = p.cast::<TestDupNode>();
				let n = node.as_mut();
				n.next = first;
				first = Some(node);
			}
			assert!(!has_cycle(first.unwrap()));
			while let Some(mut node) = first {
				let n = node.as_mut();
				let next = n.next;
				free_kernel(n as *const _ as *const _, 0);
				first = next;
			}
		}
		debug_assert_eq!(allocated_pages_count(), alloc_pages);
	}
}
