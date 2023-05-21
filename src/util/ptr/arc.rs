//! This module implements an `Arc`, similar to the one present in the Rust standard library.

use crate::errno::Errno;
use crate::memory::malloc;
use core::borrow::Borrow;
use core::marker::Unsize;
use core::mem::size_of;
use core::ops::CoerceUnsized;
use core::ops::Deref;
use core::ops::DispatchFromDyn;
use core::ptr;
use core::ptr::drop_in_place;
use core::ptr::NonNull;
use core::sync::atomic;
use core::sync::atomic::AtomicUsize;

// TODO check atomic orderings

/// Inner structure shared between arcs pointing to the same object.
pub struct ArcInner<T: ?Sized> {
	/// Strong references counter.
	strong: AtomicUsize,
	/// Weak references counter.
	weak: AtomicUsize,

	/// The object the `Arc` points to.
	obj: T,
}

/// A thread-safe reference-counting pointer. `Arc` stands for 'Atomically Reference Counted'.
pub struct Arc<T: ?Sized> {
	/// Pointer to shared object.
	ptr: NonNull<ArcInner<T>>,
}

unsafe impl<T: ?Sized + Sync + Send> Send for Arc<T> {}

unsafe impl<T: ?Sized + Sync + Send> Sync for Arc<T> {}

impl<T: ?Sized + Unsize<U>, U: ?Sized> CoerceUnsized<Arc<U>> for Arc<T> {}

impl<T: ?Sized + Unsize<U>, U: ?Sized> DispatchFromDyn<Arc<U>> for Arc<T> {}

impl<T> Arc<T> {
	/// Creates a new `Arc` for the given object.
	///
	/// This function allocates memory. On fail, it returns an error.
	pub fn new(obj: T) -> Result<Self, Errno> {
		let ptr = unsafe {
			let inner = malloc::alloc(size_of::<ArcInner<T>>())?;
			ptr::write(
				inner as *mut _,
				ArcInner {
					// The initial strong reference
					strong: AtomicUsize::new(1),
					// Every strong references collectively hold a weak reference
					weak: AtomicUsize::new(1),

					obj,
				},
			);

			NonNull::new(inner as _).unwrap()
		};

		Ok(Self {
			ptr,
		})
	}
}

impl<T: ?Sized> Arc<T> {
	/// Returns a reference to the inner object.
	fn inner(&self) -> &ArcInner<T> {
		// Safe because the inner object is Sync
		unsafe { self.ptr.as_ref() }
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
		// Drop the inner object since weak pointers cannot access it once no strong reference is
		// left
		drop_in_place(&mut Self::get_mut_unchecked(self));

		// Drop the weak reference collectively held by all strong references
		drop(Weak {
			ptr: self.ptr,
		});
	}

	/// Returns a mutable reference to the inner object without any safety check.
	pub unsafe fn get_mut_unchecked(this: &mut Arc<T>) -> &mut T {
		&mut (*this.ptr.as_ptr()).obj
	}

	/// Creates a new weak pointer to this allocation.
	pub fn downgrade(this: &Arc<T>) -> Weak<T> {
		let inner = this.inner();
		inner.weak.fetch_add(1, atomic::Ordering::Relaxed);

		Weak {
			ptr: this.ptr,
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
		inner.strong.fetch_add(1, atomic::Ordering::Relaxed);

		Self {
			ptr: self.ptr,
		}
	}
}

impl<T: ?Sized> Drop for Arc<T> {
	fn drop(&mut self) {
		let inner = self.inner();
		if inner.strong.fetch_sub(1, atomic::Ordering::Relaxed) != 1 {
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
	ptr: NonNull<ArcInner<T>>,
}

impl<T: ?Sized> Weak<T> {
	/// Returns a reference to the inner object.
	fn inner(&self) -> &ArcInner<T> {
		// Safe because the inner object is Sync
		unsafe { self.ptr.as_ref() }
	}

	/// Attempts to upgrade into an `Arc`.
	///
	/// If the value has already been dropped, the function returns `None`.
	pub fn upgrade(&self) -> Option<Arc<T>> {
		self.inner()
			.strong
			.fetch_update(atomic::Ordering::Acquire, atomic::Ordering::Relaxed, |n| {
				if n == 0 {
					return None;
				}

				Some(n + 1)
			})
			.ok()
			.map(|_| Arc {
				ptr: self.ptr,
			})
	}
}

impl<T: ?Sized> Drop for Weak<T> {
	fn drop(&mut self) {
		let inner = self.inner();
		if inner.weak.fetch_sub(1, atomic::Ordering::Relaxed) != 1 {
			return;
		}

		// Free the inner structure since it cannot be referenced anywhere else
		//
		// At this point, we can be sure the inner object has been dropped since strong references
		// collectively hold a weak reference which is removed only when the strong references
		// count reaches zero.
		unsafe {
			malloc::free(self.ptr.as_ptr() as _);
		}
	}
}
