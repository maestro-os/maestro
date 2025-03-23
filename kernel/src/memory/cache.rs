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
		buddy::{Flags, FrameOrder, ZONE_KERNEL},
		stats::MEM_INFO,
		PhysAddr, VirtAddr,
	},
	println,
	sync::{atomic::AtomicU64, mutex::Mutex},
	time::{
		clock::{current_time, CLOCK_BOOTTIME},
		unit::{TimestampScale, UTimestamp},
	},
};
use core::{
	fmt,
	fmt::Formatter,
	marker::PhantomData,
	ops::Deref,
	slice,
	sync::atomic::Ordering::{Acquire, Release},
};
use utils::{
	bytes::AnyRepr,
	collections::{btreemap::BTreeMap, list::ListNode},
	errno::{AllocResult, EResult},
	list, list_type,
	math::pow2,
	ptr::arc::Arc,
};

// TODO must be configurable
/// The timeout, in milliseconds, after which a dirty page may be written back to disk.
const WRITEBACK_TIMEOUT: u64 = 100;

/// The node from which the data of a [`RcFrame`] comes from.
#[derive(Debug)]
pub enum FrameOwner {
	/// No owner, for anonymous mappings.
	Anon,
	/// Owned by a block device.
	BlkDev(Arc<BlkDev>),
	/// Owned by a filesystem node.
	Node(Arc<Node>),
}

#[derive(Debug)]
struct RcFrameInner {
	/// Starting address of the frame
	addr: PhysAddr,
	/// The order of the frame
	order: FrameOrder,

	/// The node from which the data comes from.
	owner: FrameOwner,
	/// The offset of the data in the node in pages.
	off: u64,

	/// The node for the cache LRU.
	lru: ListNode,
	/// Timestamp of the last write to disk, in milliseconds.
	last_write: AtomicU64,
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
	/// - `owner` is the node from which the data comes from
	/// - `off` is the offset in `owner`
	pub fn new(order: FrameOrder, flags: Flags, owner: FrameOwner, off: u64) -> AllocResult<Self> {
		let addr = buddy::alloc(order, flags)?;
		Ok(Self(Arc::new(RcFrameInner {
			addr,
			order,

			owner,
			off,

			lru: Default::default(),
			last_write: AtomicU64::new(0),
		})?))
	}

	/// Allocates a new, zeroed page in the kernel zone.
	///
	/// Arguments:
	/// - `order` is the order of the buddy allocation
	/// - `owner` is the node from which the data comes from
	/// - `off` is the offset in `owner`
	pub fn new_zeroed(order: FrameOrder, owner: FrameOwner, off: u64) -> AllocResult<Self> {
		let page = Self::new(order, ZONE_KERNEL, owner, off)?;
		unsafe {
			page.slice_mut().fill(0);
		}
		Ok(page)
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
		Arc::strong_count(&self.0) > 1
	}

	/// Returns the order of the frame.
	#[inline]
	pub fn order(&self) -> FrameOrder {
		self.0.order
	}

	/// Returns the number of pages in the frame
	#[inline]
	pub fn pages_count(&self) -> usize {
		pow2(self.order() as usize)
	}

	/// Returns the offset of the frame in its associated file, if any.
	#[inline]
	pub fn offset(&self) -> u64 {
		self.0.off
	}

	/// Tells whether the frame is dirty, atomically clearing the dirty bit(s) in the virtual
	/// memory context.
	pub fn poll_dirty(&self) -> bool {
		/*if let Some(mem_space) = Process::current().mem_space.as_deref() {
			mem_space
				.lock()
				.vmem
				.poll_dirty(self.virt_addr(), self.pages_count())
		} else {
			vmem::KERNEL_VMEM
				.lock()
				.poll_dirty(self.virt_addr(), self.pages_count())
		}*/
		todo!()
	}

	/// Writes the frame's data back to disk.
	///
	/// `ts` is the timestamp at which the frame is written
	pub fn writeback(&self, ts: UTimestamp) -> EResult<()> {
		match &self.0.owner {
			FrameOwner::Anon => return Ok(()),
			FrameOwner::BlkDev(blk) => blk.ops.write_frame(self.offset(), self)?,
			FrameOwner::Node(node) => node.node_ops.writeback(self)?,
		}
		self.0.last_write.store(ts, Release);
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
pub struct PageCache {
	/// Cached frames
	///
	/// The key is the file offset, in pages, to the start of the frame
	frames: Mutex<BTreeMap<u64, RcFrame>>,
}

impl PageCache {
	/// Looks for a frame in cache at offset `off`, or reads it from `init` and inserts it in the
	/// cache.
	pub fn get_or_insert<Init: FnOnce() -> EResult<RcFrame>>(
		&self,
		off: u64,
		order: FrameOrder,
		init: Init,
	) -> EResult<RcFrame> {
		let mut frames = self.frames.lock();
		match frames.get(&off) {
			// Cache hit
			Some(frame) if frame.order() == order => Ok(frame.clone()),
			// Cache miss: read and insert
			_ => {
				let frame = init()?;
				frames.insert(off, frame.clone())?;
				Ok(frame)
			}
		}
	}
}

/// Global cache for **active** frames
static ACTIVE_LRU: Mutex<list_type!(RcFrameInner, lru)> = Mutex::new(list!(RcFrameInner, lru));
/// Global cache for **inactive** frames
static INACTIVE_LRU: Mutex<list_type!(RcFrameInner, lru)> = Mutex::new(list!(RcFrameInner, lru));

/// Inserts a frame in the inactive LRU.
pub fn insert_inactive(frame: &RcFrame) {
	// Insert the frame into the inactive LRU
	let mut lru = INACTIVE_LRU.lock();
	lru.insert_front(frame.0.clone());
	// Update statistics
	MEM_INFO.lock().inactive += frame.pages_count() * 4;
}

/// Removes a frame from the inactive LRU.
pub fn remove_inactive(frame: &RcFrame) {
	// Remove from the inactive LRU
	let mut lru = INACTIVE_LRU.lock();
	unsafe {
		lru.remove(&frame.0);
	}
	// Update statistics
	MEM_INFO.lock().inactive -= frame.pages_count() * 4;
}

fn flush_impl(lru: &mut list_type!(RcFrameInner, lru), cur_ts: UTimestamp) {
	for slot in lru.iter().rev() {
		let frame = RcFrame(slot.arc());
		if !frame.poll_dirty() {
			continue;
		}
		// If the writeback timeout is exceeded, write
		let last_write = frame.0.last_write.load(Acquire);
		if cur_ts >= last_write + WRITEBACK_TIMEOUT {
			if let Err(errno) = frame.writeback(cur_ts) {
				println!("Disk writeback I/O failure: {errno}");
			}
		}
	}
}

/// The entry point of the kernel task flushing cached memory back to disk.
pub(crate) fn flush_task() -> ! {
	sti();
	loop {
		// cannot fail since `CLOCK_BOOTTIME` is valid
		let cur_ts = current_time(CLOCK_BOOTTIME, TimestampScale::Millisecond).unwrap();
		flush_impl(&mut ACTIVE_LRU.lock(), cur_ts);
		flush_impl(&mut INACTIVE_LRU.lock(), cur_ts);
		// TODO sleep during WRITEBACK_TIMEOUT?
	}
}

/// Attempts to shrink the page cache.
///
/// If the cache cannot shrink, the function returns `false`.
pub fn shrink() -> bool {
	// Remove the last frame of the inactive cache
	let mut lru = INACTIVE_LRU.lock();
	let Some(frame) = lru.iter().next_back() else {
		return false;
	};
	let frame = RcFrame(frame.remove());
	// If dirty, write frame back to disk
	if frame.poll_dirty() {
		// No need to update the timestamp since we are removing the frame
		if let Err(errno) = frame.writeback(0) {
			println!("Disk writeback I/O failure: {errno}");
		}
	}
	// Remove the frame from its owner node
	let cache = match &frame.0.owner {
		FrameOwner::Anon => None,
		FrameOwner::BlkDev(blk) => Some(&blk.cache),
		FrameOwner::Node(node) => Some(&node.cache),
	};
	if let Some(cache) = cache {
		cache.frames.lock().remove(&frame.0.off);
	}
	// Update statistics
	MEM_INFO.lock().inactive -= frame.pages_count() * 4;
	true
}
