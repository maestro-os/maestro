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

//! Implementation of [`Arc`] and [`Weak`], similar to the ones present in the Rust standard
//! library.

use crate::{boxed::Box, errno::AllocResult};
use alloc::alloc::Global;
use core::{
	alloc::{AllocError, Allocator, Layout},
	borrow::Borrow,
	fmt,
	intrinsics::size_of_val,
	marker::Unsize,
	mem::ManuallyDrop,
	ops::{CoerceUnsized, Deref, DispatchFromDyn},
	ptr,
	ptr::{drop_in_place, NonNull},
	sync::atomic::{AtomicUsize, Ordering},
};

/// Inner structure shared between arcs pointing to the same object.
pub struct ArcInner<T: ?Sized> {
	/// Strong references counter.
	strong: AtomicUsize,
	/// Weak references counter.
	weak: AtomicUsize,
	/// The object the `Arc` points to.
	obj: T,
}

impl<T: ?Sized> ArcInner<T> {
	/// Returns the layout to allocate the structure.
	fn layout(val: &T) -> Layout {
		Layout::new::<ArcInner<()>>()
			.extend(Layout::for_value(val))
			.unwrap()
			.0
			.pad_to_align()
	}

	/// Creates an instance.
	///
	/// Arguments:
	/// - `ptr` is a pointer to the data to place in the `Arc`. This is used as a helper for memory
	/// allocation
	/// - `init` is the function to initialize the object to place in the `Arc`
	unsafe fn new<I: FnOnce(&mut T)>(ptr: *const T, init: I) -> AllocResult<NonNull<Self>> {
		// Allocate and make usable
		let inner = Global.allocate(Self::layout(&*ptr))?;
		let inner = inner.as_ptr().with_metadata_of(ptr as *const Self);
		let mut inner: NonNull<Self> = NonNull::new_unchecked(inner);
		// Initialize
		let i = inner.as_mut();
		// The initial strong reference
		i.strong = AtomicUsize::new(1);
		// Every strong references collectively hold a weak reference
		i.weak = AtomicUsize::new(1);
		init(&mut i.obj);
		Ok(inner)
	}
}

/// A thread-safe reference-counting pointer. `Arc` stands for 'Atomically Reference Counted'.
pub struct Arc<T: ?Sized> {
	/// Pointer to shared object.
	inner: NonNull<ArcInner<T>>,
}

unsafe impl<T: ?Sized + Sync + Send> Send for Arc<T> {}

unsafe impl<T: ?Sized + Sync + Send> Sync for Arc<T> {}

impl<T: ?Sized + Unsize<U>, U: ?Sized> CoerceUnsized<Arc<U>> for Arc<T> {}

impl<T: ?Sized + Unsize<U>, U: ?Sized> DispatchFromDyn<Arc<U>> for Arc<T> {}

impl<T: ?Sized> TryFrom<Box<T>> for Arc<T> {
	type Error = AllocError;

	fn try_from(obj: Box<T>) -> AllocResult<Self> {
		let inner = unsafe {
			ArcInner::new(obj.as_ptr(), |o: &mut T| {
				// Copy data
				ptr::copy_nonoverlapping(
					obj.as_ref() as *const _ as *const u8,
					o as *mut _ as *mut u8,
					size_of_val(obj.as_ref()),
				);
				// Prevent double drop
				let raw = Box::into_raw(obj);
				Box::from_raw(raw as *mut ManuallyDrop<T>);
			})?
		};

		Ok(Self {
			inner,
		})
	}
}

impl<T> Arc<T> {
	/// Creates a new `Arc` for the given object.
	///
	/// This function allocates memory. On fail, it returns an error.
	pub fn new(obj: T) -> AllocResult<Self> {
		let inner = unsafe { ArcInner::new(&obj, |o: &mut T| ptr::write(o, obj))? };
		Ok(Self {
			inner,
		})
	}

	/// Returns the inner value of the `Arc` if this is the last reference to it.
	pub fn into_inner(this: Self) -> Option<T> {
		// Avoid double free
		let this = ManuallyDrop::new(this);
		let inner = this.inner();
		if inner.strong.fetch_sub(1, Ordering::Release) != 1 {
			return None;
		}
		// If no other reference is left, get the inner value and free
		let obj = unsafe { ptr::read(&inner.obj) };
		// Drop the weak reference that is collectively held by all strong references
		drop(Weak {
			inner: this.inner,
		});
		Some(obj)
	}
}

impl<T: ?Sized> Arc<T> {
	/// Returns a reference to the inner object.
	fn inner(&self) -> &ArcInner<T> {
		// Safe because the inner object is Sync
		unsafe { self.inner.as_ref() }
	}

	/// Drops the object stored by the shared pointer.
	///
	/// This function is used when all strong references have been dropped, because the remaining
	/// weak references may not access the object once no strong reference is left.
	///
	/// # Safety
	///
	/// This function must not be called twice since it would result in a double free.
	unsafe fn partial_drop(&mut self) {
		debug_assert_eq!(Arc::strong_count(self), 0);
		// Drop the inner object since weak pointers cannot access it once no strong reference is
		// left
		drop_in_place(Self::get_mut_unchecked(self));
		// Drop the weak reference that is collectively held by all strong references
		drop(Weak {
			inner: self.inner,
		});
	}

	/// Returns a pointer to the inner object.
	pub fn as_ptr(&self) -> *const T {
		&self.inner().obj
	}

	/// Returns a mutable reference to the inner object without any safety check.
	///
	/// # Safety
	///
	/// It is the caller's responsibility to ensure concurrency rules are respected.
	#[allow(clippy::needless_pass_by_ref_mut)]
	pub unsafe fn get_mut_unchecked(this: &mut Arc<T>) -> &mut T {
		&mut (*this.inner.as_ptr()).obj
	}

	/// Returns the number of strong pointers to the allocation.
	#[inline]
	pub fn strong_count(this: &Self) -> usize {
		this.inner().strong.load(Ordering::Relaxed)
	}

	/// Returns the number of weak pointers to the allocation.
	#[inline]
	pub fn weak_count(this: &Self) -> usize {
		let weak = this.inner().weak.load(Ordering::Relaxed);
		// Subtract reference that is collectively held by strong references
		weak - 1
	}

	/// Creates a new weak pointer to this allocation.
	pub fn downgrade(this: &Arc<T>) -> Weak<T> {
		let inner = this.inner();
		inner.weak.fetch_add(1, Ordering::Relaxed);
		Weak {
			inner: this.inner,
		}
	}
}

impl<T: ?Sized> AsRef<T> for Arc<T> {
	fn as_ref(&self) -> &T {
		&self.inner().obj
	}
}

impl<T: ?Sized> Borrow<T> for Arc<T> {
	fn borrow(&self) -> &T {
		self.as_ref()
	}
}

impl<T: ?Sized> Deref for Arc<T> {
	type Target = T;

	fn deref(&self) -> &T {
		self.as_ref()
	}
}

impl<T: ?Sized> Clone for Arc<T> {
	fn clone(&self) -> Self {
		let inner = self.inner();
		let old_count = inner.strong.fetch_add(1, Ordering::Relaxed);
		if old_count == usize::MAX {
			panic!("Arc reference count overflow");
		}
		Self {
			inner: self.inner,
		}
	}
}

impl<T: ?Sized + fmt::Display> fmt::Display for Arc<T> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt::Display::fmt(&**self, f)
	}
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for Arc<T> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt::Debug::fmt(&**self, f)
	}
}

impl<T: ?Sized> Drop for Arc<T> {
	fn drop(&mut self) {
		let inner = self.inner();
		if inner.strong.fetch_sub(1, Ordering::Release) != 1 {
			return;
		}
		// Safe because this function cannot be called twice because no other `Arc` is left to
		// drop.
		unsafe {
			self.partial_drop();
		}
	}
}

/// `Weak` is a version of `Arc` that holds a non-owning reference to the managed allocation.
pub struct Weak<T: ?Sized> {
	/// Pointer to the shared object.
	inner: NonNull<ArcInner<T>>,
}

impl<T: ?Sized + Unsize<U>, U: ?Sized> CoerceUnsized<Weak<U>> for Weak<T> {}

impl<T: ?Sized + Unsize<U>, U: ?Sized> DispatchFromDyn<Weak<U>> for Weak<T> {}

impl<T: ?Sized> Weak<T> {
	/// Returns a reference to the inner object.
	fn inner(&self) -> &ArcInner<T> {
		// Safe because the inner object is Sync
		unsafe { self.inner.as_ref() }
	}

	/// Attempts to upgrade into an `Arc`.
	///
	/// If the value has already been dropped, the function returns `None`.
	pub fn upgrade(&self) -> Option<Arc<T>> {
		self.inner()
			.strong
			.fetch_update(Ordering::Acquire, Ordering::Relaxed, |n| {
				(n != 0).then_some(n + 1)
			})
			.ok()
			.map(|_| Arc {
				inner: self.inner,
			})
	}
}

impl<T: ?Sized> Clone for Weak<T> {
	fn clone(&self) -> Self {
		let inner = self.inner();
		let old_count = inner.weak.fetch_add(1, Ordering::Relaxed);
		if old_count == usize::MAX {
			panic!("Weak reference count overflow");
		}
		Self {
			inner: self.inner,
		}
	}
}

impl<T: ?Sized> Drop for Weak<T> {
	fn drop(&mut self) {
		let inner = self.inner();
		if inner.weak.fetch_sub(1, Ordering::Release) != 1 {
			return;
		}
		// Free the inner structure since it cannot be referenced anywhere else
		//
		// At this point, we can be sure the inner object has been dropped since strong references
		// collectively hold a weak reference which is removed only when the strong references
		// count reaches zero.
		let layout = Layout::for_value(self.inner());
		unsafe {
			Global.deallocate(self.inner.cast(), layout);
		}
	}
}
