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

//! The page cache allows to avoid unnecessary disk I/O by using all the available memory on the
//! system to cache the content of the disk.
//!
//! A cached frame can have the following states:
//! - **Active**: the frame is currently mapped. It cannot be reclaimed, unless the processes
//!   mapping it are killed, turning the frame inactive
//! - **Inactive**: the frame is not mapped (just in cache for a potential future use). It can be
//!   reclaimed at anytime

use crate::{
	arch::x86::sti,
	device::BlkDev,
	file::vfs::node::Node,
	memory::{
		buddy,
		buddy::{Flags, FrameOrder, Page, ZONE_KERNEL},
		stats::MEM_INFO,
		PhysAddr, VirtAddr,
	},
	println,
	sync::mutex::IntMutex,
	time::{
		clock::{current_time_ms, Clock},
		sleep_for,
		unit::{Timestamp, UTimestamp},
	},
};
use core::{
	fmt,
	fmt::Formatter,
	intrinsics::unlikely,
	marker::PhantomData,
	ops::Deref,
	slice,
	sync::atomic::Ordering::{Acquire, Release},
};
use utils::{
	bytes::AnyRepr,
	collections::{btreemap::BTreeMap, list::ListNode},
	errno::{AllocResult, EResult},
	limits::PAGE_SIZE,
	list, list_type,
	math::pow2,
	ptr::arc::Arc,
};

// TODO must be configurable
/// The timeout, in milliseconds, after which a dirty page may be written back to disk.
const WRITEBACK_TIMEOUT: u64 = 100;

/// The node from which the data of a [`RcFrame`] comes from.
#[derive(Clone, Debug)]
pub enum FrameOwner {
	/// No owner, for anonymous mappings
	Anon,
	/// Owned by a block device
	BlkDev(Arc<BlkDev>),
	/// Owned by a filesystem node
	Node(Arc<Node>),
}

impl FrameOwner {
	/// Returns a reference to the inner [`MappedNode`] if any.
	pub fn inner(&self) -> Option<&MappedNode> {
		match self {
			FrameOwner::Anon => None,
			FrameOwner::BlkDev(b) => Some(&b.mapped),
			FrameOwner::Node(n) => Some(&n.mapped),
		}
	}
}

#[derive(Debug)]
struct RcFrameInner {
	/// Starting address of the frame
	addr: PhysAddr,
	/// The order of the frame
	order: FrameOrder,

	/// The node from which the data originates
	owner: FrameOwner,

	/// The node for the cache LRU
	lru: ListNode,

	/// The device offset of the data in the node in pages
	dev_off: u64,
}

impl Drop for RcFrameInner {
	fn drop(&mut self) {
		unsafe {
			buddy::free(self.addr, self.order);
		}
	}
}

/// Reference-counted allocated physical memory frame.
///
/// When the reference count reaches zero, the frame is freed.
///
/// A new reference can be created with [`Clone`].
#[derive(Clone, Debug)]
pub struct RcFrame(Arc<RcFrameInner>);

impl RcFrame {
	/// Allocates a new, *uninitialized* frame.
	///
	/// Arguments:
	/// - `order` is the order of the buddy allocation
	/// - `flags` is the flags for the buddy allocation
	/// - `owner` is the node from which the data originates
	/// - `dev_off` is the offset of the frame on the device
	pub fn new(
		order: FrameOrder,
		flags: Flags,
		owner: FrameOwner,
		dev_off: u64,
	) -> AllocResult<Self> {
		let addr = buddy::alloc(order, flags)?;
		Ok(Self(Arc::new(RcFrameInner {
			addr,
			order,

			owner,

			lru: Default::default(),

			dev_off,
		})?))
	}

	/// Allocates a new, zeroed page in the kernel zone.
	///
	/// Arguments:
	/// - `order` is the order of the buddy allocation
	/// - `owner` is the node from which the data comes from
	/// - `dev_off` is the offset of the frame on the device
	pub fn new_zeroed(order: FrameOrder, owner: FrameOwner, dev_off: u64) -> AllocResult<Self> {
		let frame = Self::new(order, ZONE_KERNEL, owner, dev_off)?;
		unsafe {
			frame.slice_mut().fill(0);
		}
		Ok(frame)
	}

	/// Returns the page's physical address.
	#[inline]
	pub fn phys_addr(&self) -> PhysAddr {
		self.0.addr
	}

	/// Returns the page's virtual address.
	///
	/// If the address is not allocated in the kernel zone, the function panics.
	#[inline]
	pub fn virt_addr(&self) -> VirtAddr {
		self.phys_addr().kernel_to_virtual().unwrap()
	}

	/// Returns an immutable slice over the page.
	pub fn slice<T: AnyRepr>(&self) -> &[T] {
		let ptr = self.virt_addr().as_ptr::<T>();
		let len = buddy::get_frame_size(self.0.order) / size_of::<T>();
		unsafe { slice::from_raw_parts_mut(ptr, len) }
	}

	/// Returns a mutable slice.
	///
	/// # Safety
	///
	/// It is the caller's responsibility to ensure no other mutable reference exist at the same
	/// time.
	#[inline]
	#[allow(clippy::mut_from_ref)]
	pub unsafe fn slice_mut<T: AnyRepr>(&self) -> &mut [T] {
		let ptr = self.virt_addr().as_ptr::<T>();
		let len = buddy::get_frame_size(self.0.order) / size_of::<T>();
		unsafe { slice::from_raw_parts_mut(ptr, len) }
	}

	/// Tells whether there are other references to the same frame.
	#[inline]
	pub fn is_shared(&self) -> bool {
		let ref_count = Arc::strong_count(&self.0);
		match self.0.owner.inner() {
			// Anonymous mapping
			None => ref_count > 1,
			// The references in `LRU` + `PageCache` + `self` = 3
			Some(_) => ref_count > 3,
		}
	}

	/// Returns the order of the frame
	#[inline]
	pub fn order(&self) -> FrameOrder {
		self.0.order
	}

	/// Returns the number of pages in the frame
	#[inline]
	pub fn pages_count(&self) -> usize {
		pow2(self.order() as usize)
	}

	/// Returns the size of the frame in bytes
	#[inline]
	#[allow(clippy::len_without_is_empty)]
	pub fn len(&self) -> usize {
		self.pages_count() * PAGE_SIZE
	}

	/// Returns the device offset of the frame, if any.
	#[inline]
	pub fn dev_offset(&self) -> u64 {
		self.0.dev_off
	}

	/// Returns metadata for the `n`th page of the frame.
	#[inline]
	pub fn get_page(&self, n: usize) -> &'static Page {
		let addr = self.phys_addr() + n * PAGE_SIZE;
		buddy::get_page(addr)
	}

	/// Initializes the [`Page`] structures of the associated pages.
	///
	/// `off` is the offset of the frame in the associated file.
	pub fn init_pages(&self, off: u64) {
		for n in 0..self.pages_count() {
			let page = self.get_page(n);
			page.init(off + n as u64);
		}
	}

	/// Marks the `n`th page as dirty.
	pub fn mark_page_dirty(&self, n: usize) {
		self.get_page(n).dirty.store(true, Release);
	}

	/// Marks all pages on the frame as dirty.
	pub fn mark_dirty(&self) {
		for n in 0..self.pages_count() {
			self.mark_page_dirty(n);
		}
	}

	/// Writes dirty pages back to disk, if their timestamp has expired.
	///
	/// Arguments:
	/// - `ts` is the timestamp at which the frame is written. If `None`, the timestamp is ignored
	/// - `check_ts`: if `true`, pages are flushed only if the last flush is old enough (only if
	///   `ts` is specified)
	pub fn writeback(&self, ts: Option<UTimestamp>, check_ts: bool) -> EResult<()> {
		for n in 0..self.pages_count() {
			let page = self.get_page(n);
			// If not old enough, skip
			if let Some(ts) = ts {
				let last_write = page.last_write.load(Acquire);
				if check_ts && ts < last_write + WRITEBACK_TIMEOUT {
					continue;
				}
			}
			// If not dirty, skip
			if !page.dirty.swap(false, Acquire) {
				continue;
			}
			// Write page
			match &self.0.owner {
				FrameOwner::Anon => {}
				FrameOwner::BlkDev(blk) => blk.ops.write_pages(self.dev_offset(), self.slice())?,
				FrameOwner::Node(node) => node.node_ops.write_frame(node, self)?,
			}
			// Update write timestamp
			if let Some(ts) = ts {
				page.last_write.store(ts, Release);
			}
		}
		Ok(())
	}
}

/// A view over an object on a frame, where the frame is considered as an array of this object
/// type.
///
/// This structure is useful to *return* a mapped value from a function.
pub struct RcFrameVal<T: AnyRepr> {
	/// The frame the value is located on
	frame: RcFrame,
	/// The offset of the object in the array
	off: usize,
	_phantom: PhantomData<T>,
}

impl<T: AnyRepr> RcFrameVal<T> {
	/// Creates a new instance.
	pub fn new(frame: RcFrame, off: usize) -> Self {
		Self {
			frame,
			off,
			_phantom: PhantomData,
		}
	}

	/// Returns a mutable reference to the value.
	///
	/// # Safety
	///
	/// The caller must ensure no other reference to the value is living at the same time.
	#[inline]
	#[allow(clippy::mut_from_ref)]
	pub unsafe fn as_mut(&self) -> &mut T {
		&mut self.frame.slice_mut()[self.off]
	}

	/// Marks the pages storing the inner value as dirty.
	pub fn mark_dirty(&self) {
		let start = self.off / PAGE_SIZE;
		let end = (self.off + size_of::<T>()).div_ceil(PAGE_SIZE);
		for n in start..end {
			self.frame.mark_page_dirty(n);
		}
	}
}

impl<T: AnyRepr> Deref for RcFrameVal<T> {
	type Target = T;

	#[inline]
	fn deref(&self) -> &Self::Target {
		&self.frame.slice()[self.off]
	}
}

impl<T: AnyRepr + fmt::Debug> fmt::Debug for RcFrameVal<T> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		fmt::Debug::fmt(self.deref(), f)
	}
}

/// A page cache
#[derive(Debug, Default)]
pub struct MappedNode {
	/// Cached frames
	///
	/// The key is the file offset, in pages, to the start of the node
	cache: IntMutex<BTreeMap<u64, RcFrame>>,
}

impl MappedNode {
	/// Returns the frame at the offset `off`.
	///
	/// If not present, the function returns `None`.
	pub fn get(&self, off: u64) -> Option<RcFrame> {
		self.cache.lock().get(&off).cloned()
	}

	/// Looks for a frame in cache at offset `off`, or reads it from `init` and inserts it in the
	/// cache.
	pub fn get_or_insert_frame<Init: FnOnce() -> EResult<RcFrame>>(
		&self,
		off: u64,
		order: FrameOrder,
		init: Init,
	) -> EResult<RcFrame> {
		let (frame, insert) = {
			let mut frames = self.cache.lock();
			match frames.get(&off) {
				// Cache hit
				Some(frame) if frame.order() == order => (frame.clone(), false),
				// Cache miss: read and insert
				_ => {
					let frame = init()?;
					frame.init_pages(off);
					frames.insert(off, frame.clone())?;
					(frame, true)
				}
			}
		};
		// Insert in the LRU, or promote
		let mut lru = LRU.lock();
		if unlikely(insert) {
			lru.insert_front(frame.0.clone());
		} else {
			// TODO promote in the LRU. We must make sure the frame cannot be promoted by someone
			// else before being inserted
		}
		Ok(frame)
	}

	/// Synchronizes all frames in the cache back to disk.
	pub fn sync(&self) -> EResult<()> {
		let ts = current_time_ms(Clock::Boottime);
		// Sync all frames
		let frames = self.cache.lock();
		for (_, frame) in frames.iter() {
			frame.writeback(Some(ts), false)?;
		}
		Ok(())
	}

	/// Removes, without flushing, all the pages after the offset `off` (included).
	pub fn truncate(&self, off: u64) {
		let mut lru = LRU.lock();
		self.cache.lock().retain(|o, frame| {
			let retain = *o < off;
			if !retain {
				unsafe {
					lru.remove(&frame.0);
				}
			}
			retain
		});
	}
}

/// Global cache for all frames
static LRU: IntMutex<list_type!(RcFrameInner, lru)> = IntMutex::new(list!(RcFrameInner, lru));

fn flush_task_inner(cur_ts: Timestamp) {
	// Iterate on all frames
	let mut lru = LRU.lock();
	for cursor in lru.iter().rev() {
		let frame = RcFrame(cursor.arc());
		if let Err(errno) = frame.writeback(Some(cur_ts), true) {
			// Failure, try the next frame
			println!("Disk writeback I/O failure: {errno}");
			continue;
		}
	}
}

/// The entry point of the kernel task flushing cached memory back to disk.
pub(crate) fn flush_task() -> ! {
	sti();
	loop {
		let cur_ts = current_time_ms(Clock::Boottime);
		flush_task_inner(cur_ts);
		// Sleep
		let mut remain = 0;
		let _ = sleep_for(Clock::Monotonic, WRITEBACK_TIMEOUT * 1_000_000, &mut remain);
	}
}

/// Attempts to shrink the page cache.
///
/// If the cache cannot shrink, the function returns `false`.
pub fn shrink() -> bool {
	// Search for and remove an inactive frame
	let frame = {
		// Iterate, with the least recently used first
		let mut lru = LRU.lock();
		let mut iter = lru.iter().rev();
		loop {
			let Some(cursor) = iter.next() else {
				// No more frames remaining
				return false;
			};
			// Get as an Arc to access the reference counter
			let frame = RcFrame(cursor.arc());
			{
				// We lock the cache first to avoid having someone else activating the page while
				// we are removing it
				let mut cache = frame.0.owner.inner().map(|m| m.cache.lock());
				// If the frame is active, skip to the next
				if frame.is_shared() {
					continue;
				}
				if let Err(errno) = frame.writeback(None, false) {
					// Failure, try the next frame
					println!("Disk writeback I/O failure: {errno}");
					continue;
				}
				// Remove the frame from its node
				if let Some(cache) = &mut cache {
					cache.remove(&frame.0.dev_off);
				}
			}
			// Remove the frame from the LRU
			cursor.remove();
			break frame;
		}
	};
	// Update statistics
	MEM_INFO.lock().inactive -= frame.pages_count() * 4;
	true
}
