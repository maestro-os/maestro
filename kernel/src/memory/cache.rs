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
//! A cached page can have the following states:
//! - **Active**: the page is currently mapped. It cannot be reclaimed, unless the processes
//!   mapping it are killed, turning the page inactive
//! - **Inactive**: the page is not mapped (just in cache for a potential future use). It can be
//!   reclaimed at anytime

use crate::{
	device::BlkDev,
	memory::{
		PhysAddr, VirtAddr, buddy,
		buddy::{Flags, Page, ZONE_KERNEL},
		stats::MEM_INFO,
	},
	println,
	sync::{mutex::Mutex, spin::IntSpin},
	time::{
		clock::{Clock, current_time_ms},
		sleep_for,
		unit::{Timestamp, UTimestamp},
	},
};
use core::{
	fmt,
	fmt::Formatter,
	hint::unlikely,
	marker::PhantomData,
	mem,
	ops::Deref,
	slice,
	sync::atomic::{
		AtomicUsize,
		Ordering::{Acquire, Release},
	},
};
use utils::{
	bytes::AnyRepr,
	collections::{btreemap::BTreeMap, list::ListNode},
	errno::{AllocResult, EResult},
	limits::PAGE_SIZE,
	list, list_type,
	ptr::arc::Arc,
};

/// The timeout, in milliseconds, after which a dirty page may be written back to disk.
const WRITEBACK_TIMEOUT: u64 = build_cfg!(config_memory_writeback_timeout);

#[derive(Debug)]
struct RcPageInner {
	/// Address of the page
	addr: PhysAddr,

	/// The device the data lives on
	dev: Option<Arc<BlkDev>>,
	/// The device offset of the data in the node in pages
	dev_off: u64,

	/// The number of places where the page is mapped.
	map_count: AtomicUsize,
	/// The node for the cache LRU
	lru: ListNode,
}

impl Drop for RcPageInner {
	fn drop(&mut self) {
		unsafe {
			buddy::free(self.addr, 0);
		}
	}
}

/// Reference-counted allocated physical memory page.
///
/// When the reference count reaches zero, the page is freed.
///
/// A new reference can be created with [`Clone`].
#[derive(Clone, Debug)]
pub struct RcPage(Arc<RcPageInner>);

impl RcPage {
	/// Allocates a new, *uninitialized* page.
	///
	/// Arguments:
	/// - `flags` is the flags for the buddy allocation
	/// - `dev` is the device on which the data lives
	/// - `dev_off` is the offset of the page on the device
	pub fn new(flags: Flags, dev: Option<Arc<BlkDev>>, dev_off: u64) -> AllocResult<Self> {
		let addr = buddy::alloc(0, flags)?;
		Ok(Self(Arc::new(RcPageInner {
			addr,

			dev,
			dev_off,

			map_count: Default::default(),
			lru: Default::default(),
		})?))
	}

	/// Allocates a new, zeroed page in the kernel zone.
	pub fn new_zeroed() -> AllocResult<Self> {
		let page = Self::new(ZONE_KERNEL, None, 0)?;
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
		let len = PAGE_SIZE / size_of::<T>();
		unsafe { slice::from_raw_parts(ptr, len) }
	}

	/// Returns a mutable slice.
	///
	/// # Safety
	///
	/// It is the caller's responsibility to ensure no one else is accessing the content of the
	/// page at the same time.
	#[inline]
	#[allow(clippy::mut_from_ref)]
	pub unsafe fn slice_mut<T: AnyRepr>(&self) -> &mut [T] {
		let ptr = self.virt_addr().as_ptr::<T>();
		let len = PAGE_SIZE / size_of::<T>();
		unsafe { slice::from_raw_parts_mut(ptr, len) }
	}

	/// Returns the device offset of the page, if any.
	#[inline]
	pub fn dev_offset(&self) -> u64 {
		self.0.dev_off
	}

	/// Returns metadata for the `n`th page of the page.
	#[inline]
	pub fn get_page(&self) -> &'static Page {
		buddy::get_page(self.phys_addr())
	}

	/// Initializes the [`Page`] structures of the associated pages.
	///
	/// `off` is the offset of the page in the associated file.
	#[inline]
	pub fn init(&self, off: u64) {
		self.get_page().init(off);
	}

	/// Marks the page as dirty.
	pub fn mark_dirty(&self) {
		self.get_page().dirty.store(true, Release);
	}

	/// Writes dirty pages back to disk, if their timestamp has expired.
	///
	/// Arguments:
	/// - `ts` is the timestamp at which the page is written. If `None`, the timestamp is ignored
	/// - `check_ts`: if `true`, pages are flushed only if the last flush is old enough (only if
	///   `ts` is specified)
	pub fn writeback(&self, ts: Option<UTimestamp>, check_ts: bool) -> EResult<()> {
		let Some(dev) = &self.0.dev else {
			return Ok(());
		};
		let page = self.get_page();
		// If not old enough, stop
		if let Some(ts) = ts {
			let last_write = page.last_write.load(Acquire);
			if check_ts && ts < last_write + WRITEBACK_TIMEOUT {
				return Ok(());
			}
		}
		// If not dirty, stop
		if !page.dirty.swap(false, Acquire) {
			return Ok(());
		}
		// Write page
		dev.ops.writeback(dev, self.dev_offset(), self)?;
		// Update write timestamp
		if let Some(ts) = ts {
			page.last_write.store(ts, Release);
		}
		Ok(())
	}

	/// Returns a reference to the map counter.
	#[inline]
	pub fn map_counter(&self) -> &AtomicUsize {
		&self.0.map_count
	}

	/// Tells whether the page is mapped in multiple places
	#[inline]
	pub fn is_shared(&self) -> bool {
		self.0.map_count.load(Acquire) > 1
	}
}

/// A view over an object on a page, where the page is considered as an array of this object
/// type.
///
/// This structure is useful to *return* a mapped value from a function.
pub struct RcBlockVal<T: AnyRepr> {
	/// The page the value is located on
	page: RcPage,
	/// The offset of the object in the array
	off: usize,
	_phantom: PhantomData<T>,
}

impl<T: AnyRepr> RcBlockVal<T> {
	/// Creates a new instance.
	pub fn new(page: RcPage, off: usize) -> Self {
		Self {
			page,
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
		&mut self.page.slice_mut()[self.off]
	}

	/// Marks the pages storing the inner value as dirty.
	#[inline]
	pub fn mark_dirty(&self) {
		self.page.mark_dirty();
	}
}

impl<T: AnyRepr> Deref for RcBlockVal<T> {
	type Target = T;

	#[inline]
	fn deref(&self) -> &Self::Target {
		&self.page.slice()[self.off]
	}
}

impl<T: AnyRepr + fmt::Debug> fmt::Debug for RcBlockVal<T> {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		fmt::Debug::fmt(self.deref(), f)
	}
}

/// A page cache
#[derive(Debug, Default)]
pub struct MappedNode {
	/// Cached pages
	///
	/// The key is the file offset, in pages, to the start of the node
	cache: IntSpin<BTreeMap<u64, RcPage>>,
}

impl MappedNode {
	/// Returns the page at the offset `off`.
	///
	/// If not present, the function returns `None`.
	pub fn get(&self, off: u64) -> Option<RcPage> {
		self.cache.lock().get(&off).cloned()
	}

	/// Looks for a page in cache at offset `off`, or reads it from `init` and inserts it in the
	/// cache.
	pub fn get_or_insert_page<Init: FnOnce() -> EResult<RcPage>>(
		&self,
		off: u64,
		init: Init,
	) -> EResult<RcPage> {
		let (page, insert) = {
			let mut pages = self.cache.lock();
			match pages.get(&off) {
				// Cache hit
				Some(page) => (page.clone(), false),
				// Cache miss: read and insert
				_ => {
					let page = init()?;
					page.init(off);
					pages.insert(off, page.clone())?;
					(page, true)
				}
			}
		};
		// Insert in the LRU, or promote
		let mut lru = LRU.lock();
		if unlikely(insert) {
			lru.insert_front(page.0.clone());
		} else {
			// TODO promote in the LRU. We must make sure the page cannot be promoted by someone
			// else before being inserted
		}
		Ok(page)
	}

	/// Synchronizes all pages in the cache back to disk.
	pub fn sync(&self) -> EResult<()> {
		let ts = current_time_ms(Clock::Boottime);
		// Sync all pages
		let pages = self.cache.lock();
		for (_, page) in pages.iter() {
			page.writeback(Some(ts), false)?;
		}
		Ok(())
	}

	/// Removes, without flushing, all the pages after the offset `off` (included).
	pub fn truncate(&self, off: u64) {
		let mut lru = LRU.lock();
		self.cache.lock().retain(|o, page| {
			let retain = *o < off;
			if !retain {
				unsafe {
					lru.remove(&page.0);
				}
			}
			retain
		});
	}
}

impl Drop for MappedNode {
	fn drop(&mut self) {
		// Unlink all remaining pages from the LRU
		let mut lru = LRU.lock();
		let cache = mem::take(&mut self.cache).into_inner();
		for (_, page) in cache {
			unsafe {
				lru.remove(&page.0);
			}
		}
	}
}

/// Global cache for all pages
static LRU: Mutex<list_type!(RcPageInner, lru), false> = Mutex::new(list!(RcPageInner, lru));

fn flush_task_inner(cur_ts: Timestamp) {
	// Iterate on all pages
	let mut lru = LRU.lock();
	for cursor in lru.iter().rev() {
		let page = RcPage(cursor.arc());
		if let Err(errno) = page.writeback(Some(cur_ts), true) {
			// Failure, try the next page
			println!("Disk writeback I/O failure: {errno}");
			continue;
		}
	}
}

/// The entry point of the kernel task flushing cached memory back to disk.
pub(crate) fn flush_task() -> ! {
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
	// Search for and remove an inactive page
	let mut lru = LRU.lock();
	let mut iter = lru.iter().rev();
	loop {
		let Some(cursor) = iter.next() else {
			// No more pages remaining
			return false;
		};
		// Get as an Arc to access the reference counter
		let page = RcPage(cursor.arc());
		{
			// We lock the cache first to avoid having someone else activating the page while
			// we are removing it
			let mut cache = page.0.dev.as_ref().map(|dev| dev.mapped.cache.lock());
			// If the page is used somewhere else, skip to the next
			let count = 2 + cache.is_some() as usize;
			if Arc::strong_count(&page.0) > count {
				continue;
			}
			if let Err(errno) = page.writeback(None, false) {
				// Failure, try the next page
				println!("Disk writeback I/O failure: {errno}");
				continue;
			}
			// Remove the page from its node
			if let Some(cache) = &mut cache {
				cache.remove(&page.0.dev_off);
			}
		}
		// Remove the page from the LRU
		cursor.remove();
		break;
	}
	// Update statistics
	MEM_INFO.lock().inactive -= 4;
	true
}
