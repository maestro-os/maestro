//! This module contains the buddy allocator which allows to allocate `2^^n` pages large frames of
//! memory.
//!
//! This allocator works by dividing frames of memory in two until the a frame of the required size
//! is available.
//!
//! The order of a frame is the `n` in the expression `2^^n` that represents the size of a frame in
//! pages.

use core::cmp::min;
use core::ffi::c_void;
use core::mem::MaybeUninit;
use core::mem::size_of;
use crate::errno::Errno;
use crate::errno;
use crate::memory;
use crate::util::lock::*;
use crate::util::math;
use crate::util;

/// Type representing the order of a memory frame.
pub type FrameOrder = u8;
/// Type representing buddy allocator flags.
pub type Flags = i32;
/// Type representing the identifier of a frame.
type FrameID = u32;

/// The maximum order of a buddy allocated frame.
pub const MAX_ORDER: FrameOrder = 17;

/// The number of memory zones.
pub const ZONES_COUNT: usize = 3;
/// The mask for the type of the zone in buddy allocator flags.
const ZONE_TYPE_MASK: Flags = 0b11;
/// Buddy allocator flag. Tells that the allocated frame must be mapped into the user zone.
pub const FLAG_ZONE_TYPE_USER: Flags = 0b00;
/// Buddy allocator flag. Tells that the allocated frame must be mapped into the kernel zone.
pub const FLAG_ZONE_TYPE_MMIO: Flags = 0b01;
/// Buddy allocator flag. Tells that the allocated frame must be mapped into the MMIO zone.
pub const FLAG_ZONE_TYPE_KERNEL: Flags = 0b10;

/// Value indicating that the frame is used.
pub const FRAME_STATE_USED: FrameID = !0_u32;

/// Structure representing an allocatable zone of memory.
pub struct Zone {
	/// The type of the zone, defining the priority
	type_: Flags,
	/// The number of allocated pages in the zone
	allocated_pages: usize,

	/// A pointer to the beginning of the metadata of the zone
	metadata_begin: *mut c_void,
	/// A pointer to the beginning of the allocatable memory of the zone
	begin: *mut c_void,
	/// The size of the zone in bytes
	pages_count: FrameID,

	/// The free list containing linked lists to free frames
	free_list: [Option<*mut Frame>; (MAX_ORDER + 1) as usize],
}

/// Structure representing the metadata for a frame of physical memory. The structure has an
/// internal linked list for the free list. This linked list doesn't store pointers but frame
/// identifiers to save memory. If either `prev` or `next` has value `FRAME_STATE_USED`, the frame
/// is marked as used. If a frame points to itself, it means that no more elements are present in
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

/// The array of buddy allocator zones.
static mut ZONES: MaybeUninit<[IntMutex<Zone>; ZONES_COUNT]> = MaybeUninit::uninit();

/// Prepares the buddy allocator. Calling this function is required before setting the zone slots.
pub unsafe fn prepare() {
	util::zero_object(&mut ZONES);
}

/// Sets the zone at the given slot `slot`. It is required to call function `prepare` once before
/// calling this one.
/// Setting each zone slot is required before using the allocator. If one or more slot isn't set,
/// the behaviour of the allocator is undefined.
pub fn set_zone_slot(slot: usize, zone: Zone) {
	let z = unsafe {
		ZONES.assume_init_mut()
	};

	debug_assert!(slot < z.len());
	z[slot] = IntMutex::new(zone);
}

/// The size in bytes of a frame allocated by the buddy allocator with the given `order`.
#[inline(always)]
pub fn get_frame_size(order: FrameOrder) -> usize {
	memory::PAGE_SIZE << order
}

/// Returns the size of the metadata for one frame.
#[inline(always)]
pub const fn get_frame_metadata_size() -> usize {
	size_of::<Frame>()
}

/// Returns the buddy order required to fit the given number of pages.
pub fn get_order(pages: usize) -> FrameOrder {
	let mut order: FrameOrder = 0;

	while (1 << order) < pages {
		order += 1;
	}

	order
}

/// Returns a mutable reference to a zone suitable for an allocation with the given type `type_`.
fn get_suitable_zone(type_: usize) -> Option<&'static mut IntMutex<Zone>> {
	let zones = unsafe {
		ZONES.assume_init_mut()
	};

	#[allow(clippy::needless_range_loop)]
	for i in 0..zones.len() {
		let is_valid = {
			let guard = zones[i].lock();
			let zone = guard.get();
			zone.type_ == type_ as _
		};
		if is_valid {
			return Some(&mut zones[i]);
		}
	}
	None
}

/// Returns a mutable reference to the zone that contains the given pointer.
fn get_zone_for_pointer(ptr: *const c_void) -> Option<&'static mut IntMutex<Zone>> {
	let zones = unsafe {
		ZONES.assume_init_mut()
	};

	#[allow(clippy::needless_range_loop)]
	for i in 0..zones.len() {
		let is_valid = {
			let guard = zones[i].lock();
			let zone = guard.get();
			ptr >= zone.begin && (ptr as usize) < (zone.begin as usize) + zone.get_size()
		};
		if is_valid {
			return Some(&mut zones[i]);
		}
	}
	None
}

/// Allocates a frame of memory using the buddy allocator. `order` is the order of the frame to be
/// allocated. The given frame shall fit the flags `flags`. If no suitable frame is found, the
/// function returns an Err.
pub fn alloc(order: FrameOrder, flags: Flags) -> Result<*mut c_void, Errno> {
	debug_assert!(order <= MAX_ORDER);

	let begin_zone = (flags & ZONE_TYPE_MASK) as usize;
	for i in begin_zone..ZONES_COUNT {
		let z = get_suitable_zone(i);

		if let Some(z) = z {
			let mut guard = z.lock();
			let zone = guard.get_mut();

			let frame = zone.get_available_frame(order);
			if let Some(f) = frame {
				f.split(zone, order);
				f.mark_used();
				zone.allocated_pages += math::pow2(order as usize);

				let ptr = f.get_ptr(zone);
				debug_assert!(util::is_aligned(ptr, memory::PAGE_SIZE));
				debug_assert!(ptr >= zone.begin
					&& ptr < (zone.begin as usize + zone.get_size()) as _);
				return Ok(ptr);
			}
		}
	}
	Err(errno::ENOMEM)
}

/// Calls `alloc` with order `order`. The allocated frame is in the kernel zone.
/// The function returns the *virtual* address, not the physical one.
pub fn alloc_kernel(order: FrameOrder) -> Result<*mut c_void, Errno> {
	let ptr = alloc(order, FLAG_ZONE_TYPE_KERNEL)?;
	let virt_ptr = memory::kern_to_virt(ptr) as _;
	debug_assert!(virt_ptr as *const _ >= memory::PROCESS_END);

	Ok(virt_ptr)
}

/// Frees the given memory frame that was allocated using the buddy allocator. The given order must
/// be the same as the one given to allocate the frame.
pub fn free(ptr: *const c_void, order: FrameOrder) {
	debug_assert!(util::is_aligned(ptr, memory::PAGE_SIZE));
	debug_assert!(order <= MAX_ORDER);

	let z = get_zone_for_pointer(ptr).unwrap();
	let mut guard = z.lock();
	let zone = guard.get_mut();

	let frame_id = zone.get_frame_id_from_ptr(ptr);
	debug_assert!(frame_id < zone.get_pages_count());
	let frame = zone.get_frame(frame_id);
	unsafe {
		(*frame).mark_free(zone);
		(*frame).coalesce(zone);
	}
	zone.allocated_pages -= math::pow2(order as usize);
}

/// Frees the given memory frame. `ptr` is the *virtual* address to the beginning of the frame and
/// and `order` is the order of the frame.
pub fn free_kernel(ptr: *const c_void, order: FrameOrder) {
	free(memory::kern_to_phys(ptr), order);
}

/// Returns the total number of pages allocated by the buddy allocator.
pub fn allocated_pages_count() -> usize {
	let mut n = 0;

	let z = unsafe {
		ZONES.assume_init_mut()
	};
	#[allow(clippy::needless_range_loop)]
	for i in 0..z.len() {
		let guard = z[i].lock();
		n += guard.get().get_allocated_pages();
	}
	n
}

impl Zone {
	/// Fills the free list during initialization according to the number of available pages.
	fn fill_free_list(&mut self) {
		let pages_count = self.get_pages_count();
		let mut frame: FrameID = 0;
		let mut order = MAX_ORDER;

		while frame < pages_count as FrameID {
			let p = math::pow2(order as FrameID) as FrameID;
			if frame + p > pages_count {
				if order == 0 {
					break;
				}
				order -= 1;
				continue;
			}

			let f = unsafe {
				&mut *self.get_frame(frame)
			};
			f.mark_free(self);
			f.order = order;
			f.link(self);

			frame += p;
		}

		debug_assert!(frame >= pages_count);
		#[cfg(config_debug_debug)]
		self.check_free_list();
	}

	/// Creates a zone with type `type_`. The zone covers the memory from pointer `begin` to
	/// `begin + size` where `size` is the size in bytes.
	/// `metadata_begin` must be a virtual address and `begin` must be a physical address.
	pub fn new(type_: Flags, metadata_begin: *mut c_void, pages_count: FrameID, begin: *mut c_void)
		-> Zone {
		let mut z = Zone {
			type_,
			allocated_pages: 0,
			metadata_begin,
			begin,
			pages_count,
			free_list: [None; (MAX_ORDER + 1) as usize],
		};
		z.fill_free_list();
		z
	}

	/// Returns the number of allocated pages in the current zone of memory.
	pub fn get_allocated_pages(&self) -> usize {
		self.allocated_pages
	}

	/// Returns the number of allocatable pages.
	pub fn get_pages_count(&self) -> FrameID {
		self.pages_count
	}

	/// Returns the size in bytes of the allocatable memory.
	pub fn get_size(&self) -> usize {
		(self.pages_count as usize) * memory::PAGE_SIZE
	}

	/// Returns an available frame owned by this zone, with an order of at least `order`.
	fn get_available_frame(&self, order: FrameOrder) -> Option<&'static mut Frame> {
		for i in (order as usize)..self.free_list.len() {
			let f = self.free_list[i];
			if let Some(f_) = f {
				unsafe {
					debug_assert!(!(*f_).is_used());
					debug_assert!(((*f_).get_ptr(self) as usize) >= (self.begin as usize));
					debug_assert!(((*f_).get_ptr(self) as usize)
						< (self.begin as usize) + self.get_size());
				}
				return Some(unsafe { &mut *f_ });
			}
		}
		None
	}

	/// Returns the identifier for the frame at the given pointer `ptr`. The pointer must point to
	/// the frame itself, not the Frame structure.
	fn get_frame_id_from_ptr(&self, ptr: *const c_void) -> FrameID {
		(((ptr as usize) - (self.begin as usize)) / memory::PAGE_SIZE) as _
	}

	/// Returns a mutable reference to the frame with the given identifier `id`.
	/// The given identifier **must** be in the range of the zone.
	fn get_frame(&self, id: FrameID) -> *mut Frame {
		debug_assert!(id < self.get_pages_count());
		((self.metadata_begin as usize) + (id as usize * size_of::<Frame>())) as _
	}

	/// Debug function.
	/// Checks the correctness of the free list for the zone. Every frames in the free list must
	/// have an order equal to the order of the bucket it's inserted in and must be free.
	/// If a frame is the first of a list, it must not have a previous element.
	///
	/// If a frame is invalid, the function shall result in the kernel panicking.
	#[cfg(config_debug_debug)]
	fn check_free_list(&self) {
		let zone_size = (self.get_pages_count() as usize) * memory::PAGE_SIZE;

		for (order, list) in self.free_list.iter().enumerate() {
			if let Some(first) = *list {
				let mut frame = first;
				let mut is_first = true;

				loop {
					let f = unsafe {
						&*frame
					};
					let id = f.get_id(self);

					#[cfg(config_debug_debug)]
					f.check_broken(self);
					debug_assert!(!f.is_used());
					debug_assert!(f.order == (order as _));
					debug_assert!(!is_first || f.prev == id);

					let frame_ptr = f.get_ptr(self);
					debug_assert!(frame_ptr >= self.begin);
					unsafe {
						let zone_end = self.begin.add(zone_size);
						debug_assert!(frame_ptr < zone_end);
						debug_assert!(frame_ptr.add(f.get_size()) <= zone_end);
					}

					if f.next == id {
						break;
					}
					frame = self.get_frame(f.next);
					is_first = false;
				}
			}
		}
	}
}

impl Frame {
	/// Returns the id of the current frame in the associated zone `zone`.
	pub fn get_id(&self, zone: &Zone) -> FrameID {
		let self_off = self as *const _ as usize;
		let zone_off = zone.metadata_begin as *const _ as usize;
		debug_assert!(self_off >= zone_off);
		((self_off - zone_off) / size_of::<Self>()) as u32
	}

	/// Returns the pointer to the location of the associated physical memory.
	pub fn get_ptr(&self, zone: &Zone) -> *mut c_void {
		let off = self.get_id(zone) as usize * memory::PAGE_SIZE;
		(zone.begin as usize + off) as _
	}

	/// Tells whether the frame is used or not.
	pub fn is_used(&self) -> bool {
		(self.prev == FRAME_STATE_USED) || (self.next == FRAME_STATE_USED)
	}

	/// Returns the order of the frame.
	pub fn get_order(&self) -> FrameOrder {
		self.order
	}

	/// Returns the size of the frame in pages.
	pub fn get_pages(&self) -> usize {
		math::pow2(self.order as usize)
	}

	/// Returns the size of the frame in bytes.
	pub fn get_size(&self) -> usize {
		get_frame_size(self.order)
	}

	/// Marks the frame as used. The frame must not be linked to any free list.
	pub fn mark_used(&mut self) {
		self.prev = FRAME_STATE_USED;
		self.next = FRAME_STATE_USED;
	}

	/// Marks the frame as free. The frame must not be linked to any free list.
	pub fn mark_free(&mut self, zone: &Zone) {
		let id = self.get_id(zone);
		self.prev = id;
		self.next = id;
	}

	/// Debug function to assert that the chunk is valid. Invalid chunk shall result in the kernel
	/// panicking.
	#[cfg(config_debug_debug)]
	pub fn check_broken(&self, zone: &Zone) {
		debug_assert!(self.prev == FRAME_STATE_USED || self.prev < zone.get_pages_count());
		debug_assert!(self.next == FRAME_STATE_USED || self.next < zone.get_pages_count());
		debug_assert!(self.order <= MAX_ORDER);
	}

	/// Returns the identifier of the buddy frame in zone `zone`, taking in account the frame's
	/// order.
	/// The return value might be invalid and the caller has the reponsibility to check that it is
	/// below the number of frames in the zone.
	pub fn get_buddy_id(&self, zone: &Zone) -> FrameID {
		self.get_id(zone) ^ (1 << self.order) as u32
	}

	/// Links the frame into zone `zone`'s free list.
	pub fn link(&mut self, zone: &mut Zone) {
		#[cfg(config_debug_debug)]
		self.check_broken(zone);
		#[cfg(config_debug_debug)]
		zone.check_free_list();
		debug_assert!(!self.is_used());

		let id = self.get_id(zone);
		self.prev = id;
		self.next = if let Some(n) = zone.free_list[self.order as usize] {
			let next = unsafe { &mut *n };
			debug_assert!(!next.is_used());
			next.prev = id;
			next.get_id(zone)
		} else {
			id
		};
		zone.free_list[self.order as usize] = Some(self);

		#[cfg(config_debug_debug)]
		self.check_broken(zone);
		#[cfg(config_debug_debug)]
		zone.check_free_list();
	}

	/// Unlinks the frame from zone `zone`'s free list. The frame must not be used.
	pub fn unlink(&mut self, zone: &mut Zone) {
		#[cfg(config_debug_debug)]
		self.check_broken(zone);
		debug_assert!(!self.is_used());
		#[cfg(config_debug_debug)]
		zone.check_free_list();

		let id = self.get_id(zone);
		let has_prev = self.prev != id;
		let has_next = self.next != id;

		if zone.free_list[self.order as usize] == Some(self) {
			zone.free_list[self.order as usize] = if has_next {
				Some(zone.get_frame(self.next))
			} else {
				None
			};
		}

		if has_prev {
			let prev = zone.get_frame(self.prev);
			unsafe {
				(*prev).next = if has_next { self.next } else { self.prev };
			}
		}

		if has_next {
			let next = zone.get_frame(self.next);
			unsafe {
				(*next).prev = if has_prev { self.prev } else { self.next };
			}
		}

		#[cfg(config_debug_debug)]
		self.check_broken(zone);
		#[cfg(config_debug_debug)]
		zone.check_free_list();
	}

	/// Unlinks the frame from zone `zone`'s free list, splits it until it reaches the required
	/// order `order` while linking the new free frames to the free list. At the end of the
	/// function, the current frame is **not** linked to the free list.
	///
	/// The frame must not be marked as used.
	pub fn split(&mut self, zone: &mut Zone, order: FrameOrder) {
		#[cfg(config_debug_debug)]
		self.check_broken(zone);
		debug_assert!(!self.is_used());
		debug_assert!(order <= MAX_ORDER);
		debug_assert!(self.order >= order);

		self.unlink(zone);
		while self.order > order {
			self.order -= 1;

			let buddy = self.get_buddy_id(zone);
			debug_assert!(buddy != self.get_id(zone));
			if buddy >= zone.get_pages_count() {
				break;
			}

			let buddy_frame = unsafe {
				&mut *zone.get_frame(buddy)
			};
			buddy_frame.mark_free(zone);
			buddy_frame.order = self.order;
			buddy_frame.link(zone);
		}

		#[cfg(config_debug_debug)]
		self.check_broken(zone);
	}

	/// Coealesces the frame in zone `zone` with free buddy blocks recursively until no buddy is
	/// available anymore. Buddies that are merged with the frame are unlinked. The order of the
	/// frame is incremented at each merge. The frame is linked to the free list at the end.
	///
	/// The frame must not be marked as used.
	pub fn coalesce(&mut self, zone: &mut Zone) {
		#[cfg(config_debug_debug)]
		self.check_broken(zone);
		debug_assert!(!self.is_used());

		while self.order < MAX_ORDER {
			let id = self.get_id(zone);
			let buddy = self.get_buddy_id(zone);
			debug_assert!(buddy != self.get_id(zone));
			if buddy >= zone.get_pages_count() {
				break;
			}

			let new_pages_count = math::pow2((self.order + 1) as usize) as FrameID;
			if min(id, buddy) + new_pages_count > zone.get_pages_count() {
				break;
			}

			let buddy_frame = unsafe {
				&mut *zone.get_frame(buddy)
			};
			#[cfg(config_debug_debug)]
			buddy_frame.check_broken(zone);
			if buddy_frame.order != self.order || buddy_frame.is_used() {
				break;
			}

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

#[cfg(test)]
mod test {
	use super::*;
	use core::ptr::null;

	#[test_case]
	fn buddy0() {
		let alloc_pages = allocated_pages_count();

		if let Ok(p) = alloc_kernel(0) {
			unsafe {
				util::memset(p, -1, get_frame_size(0));
			}
			free_kernel(p, 0);
		} else {
			assert!(false);
		}

		debug_assert_eq!(allocated_pages_count(), alloc_pages);
	}

	#[test_case]
	fn buddy1() {
		let alloc_pages = allocated_pages_count();

		if let Ok(p) = alloc_kernel(1) {
			unsafe {
				util::memset(p, -1, get_frame_size(1));
			}
			free_kernel(p, 1);
		} else {
			assert!(false);
		}

		debug_assert_eq!(allocated_pages_count(), alloc_pages);
	}

	fn lifo_test(i: usize) {
		if let Ok(p) = alloc_kernel(0) {
			unsafe {
				util::memset(p, -1, get_frame_size(0));
			}
			if i > 0 {
				lifo_test(i - 1);
			}
			free_kernel(p, 0);
		} else {
			assert!(false);
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

		let mut frames: [*const c_void; 100] = [null::<c_void>(); 100];

		for i in 0..frames.len() {
			if let Ok(p) = alloc_kernel(0) {
				frames[i] = p;
			} else {
				assert!(false);
			}
		}

		for frame in frames.iter() {
			free_kernel(*frame, 0);
		}

		debug_assert_eq!(allocated_pages_count(), alloc_pages);
	}

	fn get_dangling(order: FrameOrder) -> *mut c_void {
		if let Ok(p) = alloc_kernel(order) {
			unsafe {
				util::memset(p, -1, get_frame_size(order));
			}
			free_kernel(p, 0);
			p
		} else {
			assert!(false);
			null::<c_void>() as _
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
		next: *mut TestDupNode,
	}

	fn has_cycle(begin: *const TestDupNode) -> bool {
		if begin != null::<TestDupNode>() as _ {
			return false;
		}

		let mut tortoise = begin;
		let mut hoare = unsafe { (*begin).next };
		while (tortoise != null::<TestDupNode>() as _)
			&& (hoare != null::<TestDupNode>() as _)
			&& (tortoise != hoare) {
			tortoise = unsafe { (*tortoise).next };

			if unsafe { (*hoare).next } != null::<TestDupNode>() as _ {
				return false;
			}
			hoare = unsafe { (*(*hoare).next).next };
		}
		tortoise == hoare
	}

	#[test_case]
	fn buddy_full_duplicate() {
		let alloc_pages = allocated_pages_count();

		let mut first = null::<TestDupNode>() as *mut TestDupNode;
		while let Ok(p) = alloc_kernel(0) {
			let node = p as *mut TestDupNode;
			unsafe {
				(*node).next = first;
			}
			first = node;
			assert!(!has_cycle(first));
		}

		while first != null::<TestDupNode>() as _ {
			let next = unsafe { (*first).next };
			free_kernel(first as _, 0);
			first = next;
		}

		debug_assert_eq!(allocated_pages_count(), alloc_pages);
	}
}
