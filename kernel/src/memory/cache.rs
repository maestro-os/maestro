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
		vmem, PhysAddr, VirtAddr,
	},
	println,
	process::Process,
	sync::{atomic::AtomicU64, mutex::Mutex},
	time::{
		clock::{current_time, CLOCK_BOOTTIME},
		unit::{TimestampScale, UTimestamp},
	},
};
use core::{
	fmt,
	fmt::Formatter,
	intrinsics::unlikely,
	marker::PhantomData,
	ops::Deref,
	slice,
	sync::atomic::{
		AtomicBool,
		Ordering::{Acquire, Release},
	},
};
use utils::{
	bytes::AnyRepr,
	collections::{btreemap::BTreeMap, list::ListNode},
	concurrent_copy,
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
	/// Tells whether the frame has been written to
	dirty: AtomicBool,
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
			dirty: AtomicBool::new(false),
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

	/// Allocates a new frame owned by `node` and copies the content of `self` to it.
	pub fn duplicate(&self, node: &Arc<Node>) -> AllocResult<Self> {
		let frame = Self::new(
			self.order(),
			ZONE_KERNEL,
			FrameOwner::Node(node.clone()),
			self.offset(),
		)?;
		concurrent_copy(self.slice(), frame.slice());
		Ok(frame)
	}

	/// Tells whether the frame has been accessed and is dirty, atomically clearing the dirty bits
	/// in the virtual memory context.
	///
	/// Returns values:
	/// - Whether the frame has been accessed
	/// - Whether the frame has been written to (is dirty)
	pub fn poll_access(&self) -> (bool, bool) {
		if let Some(mem_space) = Process::current().mem_space.as_deref() {
			mem_space
				.lock()
				.vmem
				.poll_access(self.virt_addr(), self.pages_count())
		} else {
			vmem::KERNEL_VMEM
				.lock()
				.poll_access(self.virt_addr(), self.pages_count())
		}
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
		let (frame, insert) = {
			let mut frames = self.frames.lock();
			match frames.get(&off) {
				// Cache hit
				Some(frame) if frame.order() == order => (frame.clone(), false),
				// Cache miss: read and insert
				_ => {
					let frame = init()?;
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
}

/// Global cache for all frames
static LRU: Mutex<list_type!(RcFrameInner, lru)> = Mutex::new(list!(RcFrameInner, lru));

/// The entry point of the kernel task flushing cached memory back to disk.
pub(crate) fn flush_task() -> ! {
	sti();
	loop {
		// cannot fail since `CLOCK_BOOTTIME` is valid
		let cur_ts = current_time(CLOCK_BOOTTIME, TimestampScale::Millisecond).unwrap();
		// Iterate on all frames
		let mut lru = LRU.lock();
		for mut slot in lru.iter().rev() {
			let frame = RcFrame(slot.arc());
			let (accessed, dirty) = frame.poll_access();
			// No need to check `dirty` since a dirty page is automatically accessed
			if accessed {
				slot.lru_promote();
			}
			// If the frame has not been written to, nothing else to do
			if !dirty && !frame.0.dirty.load(Acquire) {
				continue;
			}
			// If the writeback timeout is exceeded, write
			let last_write = frame.0.last_write.load(Acquire);
			if cur_ts >= last_write + WRITEBACK_TIMEOUT {
				match frame.writeback(cur_ts) {
					// On success, clear the dirty flag
					Ok(_) => {
						frame.0.dirty.fetch_and(false, Release);
					}
					Err(errno) => println!("Disk writeback I/O failure: {errno}"),
				}
			} else {
				// Set the dirty flag for later writeback
				frame.0.dirty.fetch_or(true, Release);
			}
		}
		// TODO sleep during WRITEBACK_TIMEOUT?
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
			let Some(mut cursor) = iter.next() else {
				// No more frames remaining
				return false;
			};
			// Get as an Arc to access the reference counter
			let frame = RcFrame(cursor.arc());
			{
				// We lock the cache first to avoid having someone else activating the page while
				// we are removing it
				let cache = match &frame.0.owner {
					FrameOwner::Anon => None,
					FrameOwner::BlkDev(blk) => Some(&blk.cache),
					FrameOwner::Node(node) => Some(&node.cache),
				};
				let mut cache = cache.map(|c| c.frames.lock());
				// If the frame is active, skip to the next. The references in `LRU` + `PageCache`
				// + `frame` = 3
				if Arc::strong_count(&frame.0) > 3 {
					continue;
				}
				// Remove the frame from its owner node
				if let Some(cache) = &mut cache {
					// If dirty, write the frame back to disk
					let (_, dirty) = frame.poll_access();
					if dirty {
						// No need to update the timestamp since we are removing the frame
						if let Err(errno) = frame.writeback(0) {
							// On error, jump to the next frame to avoid loosing data. Also promote
							// it in the LRU to reduce the likelihood of meeting it at the next
							// call
							println!("Disk writeback I/O failure: {errno}");
							cursor.lru_promote();
							continue;
						}
					}
					cache.remove(&frame.0.off);
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
