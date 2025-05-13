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

//! Implementation of [`Arc`], similar to the ones present in the Rust standard
//! library, but without the support for `Weak`.

use crate::{__alloc, __dealloc, boxed::Box, errno::AllocResult};
use core::{
	alloc::{AllocError, Layout},
	borrow::Borrow,
	fmt,
	hash::{Hash, Hasher},
	marker::Unsize,
	mem,
	mem::{ManuallyDrop, offset_of},
	ops::{CoerceUnsized, Deref, DispatchFromDyn},
	ptr,
	ptr::{NonNull, drop_in_place, null, null_mut},
	sync::atomic::{AtomicPtr, AtomicUsize, Ordering, Ordering::Relaxed},
};

/// Inner structure shared between arcs pointing to the same object.
pub struct ArcInner<T: ?Sized> {
	/// References counter.
	pub ref_count: AtomicUsize,
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
	///   allocation
	/// - `init` is the function to initialize the object to place in the `Arc`
	unsafe fn new<I: FnOnce(&mut T)>(ptr: *const T, init: I) -> AllocResult<NonNull<Self>> {
		// Allocate and make usable
		let inner = __alloc(Self::layout(&*ptr))?;
		let inner = inner.as_ptr().with_metadata_of(ptr as *const Self);
		let mut inner: NonNull<Self> = NonNull::new_unchecked(inner);
		// Initialize
		let i = inner.as_mut();
		// The initial strong reference
		i.ref_count = AtomicUsize::new(1);
		init(&mut i.obj);
		Ok(inner)
	}
}

/// A thread-safe reference-counting pointer. `Arc` stands for 'Atomically Reference Counted'.
pub struct Arc<T: ?Sized> {
	/// Pointer to shared object.
	pub inner: NonNull<ArcInner<T>>,
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

	/// Constructs an `Arc<T>` from a raw pointer.
	///
	/// # Safety
	///
	/// The raw pointer must have been previously returned by a call to [`Arc<T>::into_raw`]. Else,
	/// the behaviour is undefined.
	pub unsafe fn from_raw(ptr: *const T) -> Arc<T> {
		let off = offset_of!(ArcInner<T>, obj);
		Arc {
			inner: unsafe { NonNull::new_unchecked(ptr.byte_sub(off) as *mut ArcInner<T>) },
		}
	}

	/// Consumes the `Arc`, returning the wrapped pointer.
	///
	/// To avoid a memory leak, the pointer must be converted back to an `Arc` using
	/// [`Arc::from_raw`].
	pub fn into_raw(this: Arc<T>) -> *const T {
		let ptr = this.as_ref() as *const T;
		mem::forget(this);
		ptr
	}

	/// Returns the inner value of the `Arc` if this is the last reference to it.
	pub fn into_inner(this: Self) -> Option<T> {
		// Avoid double free
		let this = ManuallyDrop::new(this);
		let inner = this.inner();
		if inner.ref_count.fetch_sub(1, Ordering::Release) != 1 {
			return None;
		}
		unsafe {
			// If no other reference is left, get the inner value and free
			let obj = ptr::read(&inner.obj);
			// Free the inner structure without dropping the object since it has been read before
			let layout = Layout::for_value(inner);
			__dealloc(this.inner.cast(), layout);
			Some(obj)
		}
	}
}

impl<T: ?Sized> Arc<T> {
	/// Returns a reference to the inner object.
	fn inner(&self) -> &ArcInner<T> {
		// Safe because the inner object is Sync
		unsafe { self.inner.as_ref() }
	}

	/// Returns a pointer to the inner object.
	pub fn as_ptr(this: &Self) -> *const T {
		&this.inner().obj
	}

	/// Returns a mutable reference into the given `Arc`, if there are no other `Arc` pointers to
	/// the same allocation.
	pub fn as_mut(this: &mut Self) -> Option<&mut T> {
		// Cannot have a race condition since `this` is mutably borrowed
		if Arc::strong_count(this) == 1 {
			Some(unsafe { &mut this.inner.as_mut().obj })
		} else {
			None
		}
	}

	/// Returns the number of strong pointers to the allocation.
	#[inline]
	pub fn strong_count(this: &Self) -> usize {
		this.inner().ref_count.load(Ordering::Relaxed)
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
		let old_count = inner.ref_count.fetch_add(1, Ordering::Relaxed);
		if old_count == usize::MAX {
			panic!("Arc reference count overflow");
		}
		Self {
			inner: self.inner,
		}
	}
}

impl<T: Eq> Eq for Arc<T> {}

impl<T: PartialEq> PartialEq for Arc<T> {
	fn eq(&self, other: &Self) -> bool {
		Self::as_ref(self).eq(other)
	}
}

impl<T: Hash> Hash for Arc<T> {
	fn hash<H: Hasher>(&self, state: &mut H) {
		Self::as_ref(self).hash(state)
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
		if inner.ref_count.fetch_sub(1, Ordering::Release) != 1 {
			return;
		}
		unsafe {
			// Drop the object
			let obj = &mut (*self.inner.as_ptr()).obj;
			drop_in_place(obj);
			// Free the inner structure
			let layout = Layout::for_value(inner);
			__dealloc(self.inner.cast(), layout);
		}
	}
}

/// Relaxed atomic [`Arc`] storage.
#[derive(Default)]
pub struct RelaxedArcCell<T>(AtomicPtr<T>);

impl<T> From<Arc<T>> for RelaxedArcCell<T> {
	fn from(val: Arc<T>) -> Self {
		let ptr = Arc::into_raw(val);
		Self(AtomicPtr::new(ptr as _))
	}
}

impl<T> RelaxedArcCell<T> {
	/// Creates a new instance.
	#[inline]
	pub const fn new() -> Self {
		Self(AtomicPtr::new(null_mut()))
	}

	/// Get a copy of the inner [`Arc`].
	pub fn get(&self) -> Option<Arc<T>> {
		let ptr = self.0.load(Relaxed);
		(!ptr.is_null()).then(|| {
			let arc = unsafe { Arc::from_raw(ptr) };
			// Increment reference counter
			mem::forget(arc.clone());
			arc
		})
	}

	/// Swaps the inner value for `val`, returning the previous.
	pub fn replace(&self, val: Option<Arc<T>>) -> Option<Arc<T>> {
		let new = val.map(Arc::into_raw).unwrap_or(null());
		let old = self.0.swap(new as _, Relaxed);
		(!old.is_null()).then(|| unsafe { Arc::from_raw(old) })
	}

	/// Set the inner [`Arc`].
	#[inline]
	pub fn set(&self, val: Option<Arc<T>>) {
		self.replace(val);
	}
}
