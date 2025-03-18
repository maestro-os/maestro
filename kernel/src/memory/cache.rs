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
	memory::{
		buddy,
		buddy::{Flags, FrameOrder, ZONE_KERNEL},
		PhysAddr, VirtAddr,
	},
	sync::mutex::Mutex,
};
use core::{fmt, fmt::Formatter, marker::PhantomData, ops::Deref, slice};
use utils::{
	bytes::AnyRepr,
	collections::btreemap::BTreeMap,
	errno::{AllocResult, EResult},
	math::pow2,
	ptr::arc::Arc,
};

#[derive(Debug)]
struct RcFrameInner {
	/// Starting address of the frame
	addr: PhysAddr,
	/// The order of the frame
	order: FrameOrder,
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
	pub fn new(order: FrameOrder, flags: Flags) -> AllocResult<Self> {
		let addr = buddy::alloc(order, flags)?;
		Ok(Self(Arc::new(RcFrameInner {
			addr,
			order,
		})?))
	}

	/// Allocates a new, zeroed page in the kernel zone.
	pub fn new_zeroed(order: FrameOrder) -> AllocResult<Self> {
		let page = Self::new(order, ZONE_KERNEL)?;
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
