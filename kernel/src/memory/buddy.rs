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

use super::{stats, PhysAddr, VirtAddr};
use crate::{file::vfs::node::Node, sync::mutex::IntMutex};
use core::{
	alloc::AllocError,
	intrinsics::likely,
	mem::{offset_of, size_of},
	ptr,
	ptr::{null_mut, NonNull},
	slice,
	sync::atomic::AtomicBool,
};
use utils::{errno::AllocResult, limits::PAGE_SIZE, math, ptr::arc::Arc};

/// The order of a memory frame.
pub type FrameOrder = u8;
/// Buddy allocator flags.
pub type Flags = i32;
// An `u32` is enough to fit 16 TiB of RAM
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

/// The size of the metadata for one frame.
pub const FRAME_METADATA_SIZE: usize = size_of::<Frame>();

/// An allocatable zone of memory, initialized at boot.
pub(crate) struct Zone {
	/// A pointer to the beginning of the metadata of the zone
	metadata_begin: *mut Frame,
	/// A pointer to the beginning of the allocatable memory of the zone
	begin: PhysAddr,
	/// The size of the zone in pages
	pages_count: FrameID,
	/// The number of allocated pages in the zone
	allocated_pages: usize,
	/// The free list containing linked lists to free frames. Each linked list contain frames of
	/// the order corresponding to the element in this array
	free_list: [Option<NonNull<FreeFrame>>; (MAX_ORDER + 1) as usize],
}

impl Zone {
	/// Returns a value for use as a placeholder until boot-time initialization has been performed.
	const fn placeholder() -> Self {
		Self {
			metadata_begin: null_mut(),
			begin: PhysAddr(0),
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
		// Init all frames to avoid undefined values
		for f in frames.iter_mut() {
			unsafe {
				ptr::write(f, Frame::Free(Default::default()));
			}
		}
		// Init free lists
		let mut i: FrameID = 0;
		let mut order = MAX_ORDER;
		while i < self.pages_count as FrameID {
			// Check the order fits in remaining pages
			let len = math::pow2(order as FrameID) as FrameID;
			if i + len > self.pages_count {
				order -= 1;
				continue;
			}
			// Init frame
			let frame = &mut frames[i as usize];
			let free_frame = frame.mark_free(order);
			free_frame.link(self);
			// Jump to next offset
			i += len;
		}
	}

	/// Creates a buddy allocator zone.
	///
	/// The zone covers the memory from pointer `begin` to `begin + size` where `size` is the size
	/// in bytes.
	///
	/// `metadata_begin` must be a virtual address and `begin` must be a
	/// physical address.
	pub(crate) fn new(metadata_begin: VirtAddr, begin: PhysAddr, pages_count: FrameID) -> Zone {
		let mut z = Zone {
			metadata_begin: metadata_begin.as_ptr(),
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
		(self.pages_count as usize) * PAGE_SIZE
	}

	/// Returns an available frame owned by this zone, with an order of at least
	/// `order`.
	fn get_available_frame(&mut self, order: FrameOrder) -> Option<NonNull<FreeFrame>> {
		self.free_list[(order as usize)..].iter().find_map(|f| *f)
	}

	/// Returns the identifier for the frame at the given physical address.
	///
	/// The pointer must point to the frame itself, not the Frame structure.
	fn get_frame_id_from_addr(&self, addr: PhysAddr) -> FrameID {
		((addr.0 - self.begin.0) / PAGE_SIZE) as _
	}

	/// Returns a mutable slice over the metadata of the zone's frames.
	#[inline]
	fn frames(&self) -> &'static mut [Frame] {
		unsafe { slice::from_raw_parts_mut(self.metadata_begin, self.pages_count as usize) }
	}
}

/// Returns the ID of `frame` in the associated zone `zone`.
///
/// # Safety
///
/// `frame` must be a pointer to either [`Frame`] itself, [`FreeFrame`] or [`PageState`].
unsafe fn frame_id<T>(zone: &Zone, frame: &T) -> FrameID {
	let self_off = frame as *const _ as usize;
	let Some(off) = self_off.checked_sub(zone.metadata_begin as usize) else {
		unreachable!();
	};
	(off / size_of::<Frame>()) as u32
}

/// Free frame linked list.
#[derive(Debug, Default)]
struct FreeFrame {
	/// Previous frame in the free list.
	prev: Option<NonNull<Self>>,
	/// Next frame in the free list.
	next: Option<NonNull<Self>>,
	/// Order of the frame, used to check the size of the matching buddy when coalescing.
	order: FrameOrder,
}

impl FreeFrame {
	/// Returns a mutable reference to the wrapping [`Frame`].
	fn frame(&mut self) -> &mut Frame {
		let off = offset_of!(Frame, Free.0);
		unsafe { &mut *(self as *mut _ as *mut Frame).byte_sub(off) }
	}

	/// Returns the index of the buddy frame in zone `zone`.
	///
	/// `order` is the order of the `self` frame.
	///
	/// The caller has the responsibility to check that it is below the number of frames in the
	/// zone.
	#[inline]
	fn get_buddy_id(&self, zone: &Zone) -> FrameID {
		let id = unsafe { frame_id(zone, self) };
		id ^ math::pow2(self.order as u32)
	}

	/// Links the frame into zone `zone`'s free list of order `order`.
	fn link(&mut self, zone: &mut Zone) {
		let order = self.order;
		self.prev = None;
		self.next = zone.free_list[order as usize];
		if let Some(mut next) = self.next {
			let next = unsafe { next.as_mut() };
			next.prev = NonNull::new(self);
		}
		zone.free_list[order as usize] = NonNull::new(self);
	}

	/// Unlinks the frame from zone `zone`'s free list of order `order`.
	fn unlink(&mut self, zone: &mut Zone) {
		if let Some(mut prev) = self.prev {
			let prev = unsafe { &mut prev.as_mut() };
			prev.next = self.next;
		} else {
			// First element of the list: update it
			zone.free_list[self.order as usize] = self.next;
		}
		if let Some(mut next) = self.next {
			let next = unsafe { &mut next.as_mut() };
			next.prev = self.prev;
		}
	}

	/// Unlinks the frame from zone `zone`'s free list, splits it until it
	/// reaches the required order `order` while linking the new free frames to
	/// the free list.
	///
	/// At the end of the function, the current frame is **not** linked to the free list.
	///
	/// The frame must not be marked as used.
	fn split(&mut self, zone: &mut Zone, order: FrameOrder) {
		debug_assert!(order <= MAX_ORDER);
		let frames = zone.frames();
		self.unlink(zone);
		while self.order > order {
			self.order -= 1;
			// Get buddy
			let buddy = self.get_buddy_id(zone);
			let buddy_frame = &mut frames[buddy as usize];
			let free_buddy = buddy_frame.mark_free(self.order);
			free_buddy.link(zone);
		}
	}

	/// Coalesces the frame in zone `zone` with free buddy blocks recursively
	/// until no buddy is available anymore.
	///
	/// The current frame must be free.
	///
	/// Buddies that are merged with the frame are unlinked.
	///
	/// The order of the frame is incremented at each merge.
	///
	/// The frame is linked to the free list by the function.
	fn coalesce(&mut self, zone: &mut Zone) {
		let frames = zone.frames();
		while self.order < MAX_ORDER {
			let id = unsafe { frame_id(zone, self) };
			// Get buddy ID
			let buddy = self.get_buddy_id(zone);
			// Check if coalesce is possible
			if buddy >= zone.pages_count {
				break;
			}
			let buddy_frame = &mut frames[buddy as usize];
			let Frame::Free(free_buddy) = buddy_frame else {
				break;
			};
			if free_buddy.order != self.order {
				break;
			}
			// Update buddy
			free_buddy.unlink(zone);
			if id < buddy {
				self.order += 1;
			} else {
				free_buddy.order += 1;
				free_buddy.coalesce(zone);
				return;
			}
		}
		self.link(zone);
	}
}

/// State of a physical page.
#[derive(Debug, Default)]
pub struct PageState {
	/// The mapped node, if any.
	node: Option<Arc<Node>>,
	/// The offset of the page in the node. If not in a node, the value is irrelevant.
	index: u64,
	/// Flag indicating whether the page needs synchronization to the backing store.
	dirty: AtomicBool,
}

impl PageState {
	/// Returns the address of the associated physical memory.
	fn addr(&self, zone: &Zone) -> PhysAddr {
		let id = unsafe { frame_id(zone, self) };
		zone.begin + id as usize * PAGE_SIZE
	}
}

/// The metadata for a frame of physical memory.
enum Frame {
	/// The frame is free.
	Free(FreeFrame),
	/// The frame is allocated.
	Allocated(PageState),
}

impl Frame {
	/// Tells whether the frame is used or not.
	#[inline]
	fn is_allocated(&self) -> bool {
		matches!(self, Frame::Allocated(_))
	}

	/// Marks the frame as free. The frame must not be linked to any free list.
	///
	/// The function returns the free representation of the frame.
	#[inline]
	fn mark_free(&mut self, order: FrameOrder) -> &mut FreeFrame {
		*self = Frame::Free(FreeFrame {
			prev: None,
			next: None,
			order,
		});
		match self {
			Frame::Free(f) => f,
			_ => unreachable!(),
		}
	}

	/// Marks the frame as used. The frame must not be linked to any free list.
	///
	/// The function returns the used representation of the frame.
	#[inline]
	fn mark_used(&mut self) -> &PageState {
		*self = Frame::Allocated(Default::default());
		match self {
			Frame::Allocated(f) => f,
			_ => unreachable!(),
		}
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
	PAGE_SIZE << order
}

/// Returns the buddy order required to fit the given number of pages.
#[inline]
pub fn get_order(pages: usize) -> FrameOrder {
	// this is equivalent to `ceil(log2(pages))`
	if likely(pages != 0) {
		(usize::BITS - pages.leading_zeros()) as _
	} else {
		0
	}
}

/// Returns a mutable reference to the zone that contains the given physical address `phys_addr`.
///
/// `zones` is the list of zones.
fn get_zone_for_addr(zones: &mut [Zone; ZONES_COUNT], phys_addr: PhysAddr) -> Option<&mut Zone> {
	zones.iter_mut().find(|z| {
		let end = z.begin + z.get_size();
		(z.begin..end).contains(&phys_addr)
	})
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
pub fn alloc(order: FrameOrder, flags: Flags) -> AllocResult<PhysAddr> {
	if order > MAX_ORDER {
		return Err(AllocError);
	}
	// Select a zone and frame to allocate on
	let mut zones = ZONES.lock();
	let begin_zone = (flags & ZONE_TYPE_MASK) as usize;
	let (mut frame, zone) = zones[begin_zone..]
		.iter_mut()
		.find_map(|z| Some((z.get_available_frame(order)?, z)))
		.ok_or(AllocError)?;
	let frame = unsafe { frame.as_mut() };
	// Do the actual allocation
	frame.split(zone, order);
	let state = frame.frame().mark_used();
	let addr = state.addr(zone);
	debug_assert!(addr >= zone.begin && addr < zone.begin + zone.get_size());
	// Statistics
	let pages_count = math::pow2(order as usize);
	zone.allocated_pages += pages_count;
	stats::MEM_INFO.lock().mem_free -= pages_count * 4;
	#[cfg(feature = "memtrace")]
	super::trace::sample("buddy", super::trace::SampleOp::Alloc, addr.0, pages_count);
	Ok(addr)
}

/// Calls [`alloc()`] with order `order`, allocating in the kernel zone.
///
/// The function returns the virtual address, to the frame.
pub fn alloc_kernel(order: FrameOrder) -> AllocResult<NonNull<u8>> {
	alloc(order, FLAG_ZONE_TYPE_KERNEL)?
		.kernel_to_virtual()
		.and_then(|addr| NonNull::new(addr.as_ptr()))
		.ok_or(AllocError)
}

/// Returns the instance of [`PageState`] associated with the page at `addr`.
///
/// If the page is not allocated, the function panics.
pub fn page_state(addr: PhysAddr) -> &'static PageState {
	debug_assert!(addr.is_aligned_to(PAGE_SIZE));
	// Get zone
	let mut zones = ZONES.lock();
	let zone = get_zone_for_addr(&mut zones, addr).unwrap();
	let frames = zone.frames();
	// Get frame
	let frame_id = zone.get_frame_id_from_addr(addr);
	debug_assert!(frame_id < zone.pages_count);
	let frame = &frames[frame_id as usize];
	let Frame::Allocated(state) = frame else {
		panic!("attempt to retrieve the state of a free page");
	};
	state
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
pub unsafe fn free(addr: PhysAddr, order: FrameOrder) {
	debug_assert!(addr.is_aligned_to(PAGE_SIZE));
	debug_assert!(order <= MAX_ORDER);
	// Get zone
	let mut zones = ZONES.lock();
	let zone = get_zone_for_addr(&mut zones, addr).unwrap();
	let frames = zone.frames();
	// Perform free
	let frame_id = zone.get_frame_id_from_addr(addr);
	debug_assert!(frame_id < zone.pages_count);
	let frame = &mut frames[frame_id as usize];
	debug_assert!(frame.is_allocated());
	let free_frame = frame.mark_free(order);
	free_frame.coalesce(zone);
	// Statistics
	let pages_count = math::pow2(order as usize);
	zone.allocated_pages -= pages_count;
	stats::MEM_INFO.lock().mem_free += pages_count * 4;
	#[cfg(feature = "memtrace")]
	super::trace::sample("buddy", super::trace::SampleOp::Free, addr.0, pages_count);
}

/// Frees the given memory frame.
///
/// Arguments:
/// - `ptr` is the pointer to the beginning of the frame
/// - `order` is the order of the frame
///
/// # Safety
///
/// See [`free`]
pub unsafe fn free_kernel(ptr: *mut u8, order: FrameOrder) {
	let addr = VirtAddr::from(ptr).kernel_to_physical().unwrap();
	free(addr, order);
}

/// Returns the total number of pages allocated by the buddy allocator.
pub fn allocated_pages_count() -> usize {
	let zones = ZONES.lock();
	zones.iter().map(|z| z.allocated_pages).sum()
}

#[cfg(test)]
mod test {
	use super::*;

	#[test_case]
	fn buddy0() {
		let alloc_pages = allocated_pages_count();
		unsafe {
			let p = alloc_kernel(0).unwrap();
			let slice = slice::from_raw_parts_mut(p.as_ptr(), get_frame_size(0));
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
			let slice = slice::from_raw_parts_mut(p.as_ptr(), get_frame_size(0));
			slice.fill(!0);
			free_kernel(p.as_ptr(), 1);
		}
		debug_assert_eq!(allocated_pages_count(), alloc_pages);
	}

	fn lifo_test(i: usize) {
		unsafe {
			let p = alloc_kernel(0).unwrap();
			let slice = slice::from_raw_parts_mut(p.as_ptr(), get_frame_size(0));
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
		let mut frames: [PhysAddr; 100] = [PhysAddr(0); 100];
		unsafe {
			for frame in &mut frames {
				*frame = alloc(0, FLAG_ZONE_TYPE_KERNEL).unwrap();
			}
			for frame in frames {
				free(frame, 0);
			}
		}
		debug_assert_eq!(allocated_pages_count(), alloc_pages);
	}

	fn get_dangling(order: FrameOrder) -> *mut u8 {
		unsafe {
			let p = alloc_kernel(order).unwrap();
			let slice = slice::from_raw_parts_mut(p.as_ptr(), get_frame_size(0));
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
				free_kernel(n as *mut _ as *mut _, 0);
				first = next;
			}
		}
		debug_assert_eq!(allocated_pages_count(), alloc_pages);
	}
}
