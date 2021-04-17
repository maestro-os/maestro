/// This module contains pointer-like structures.

use core::marker::Unsize;
use core::mem::size_of;
use core::ops::CoerceUnsized;
use core::ops::DispatchFromDyn;
use core::ops::{Deref, DerefMut};
use core::ptr::NonNull;
use core::ptr::drop_in_place;
use crate::errno::Errno;
use crate::memory::malloc;
use crate::util::write_ptr;

/// Inner structure of the shared pointer. The same instance of this structure is shared with
/// every clones of a SharedPtr structure. This structure holds the number of SharedPtr holding it.
/// Each time the pointer is cloned, the counter is incremented. Each time a copy is dropped, the
/// counter is decrementer. The inner structure is dropped at the moment the counter reaches `0`.
#[derive(Debug)]
struct SharedPtrInner<T: ?Sized> {
	/// The nubmer of shared pointers holding the structure.
	count: usize,
	/// The object stored by the structure.
	obj: T,
}

impl<T> SharedPtrInner<T> {
	/// Creates a new instance with the given object. The counter is initialized to `1`.
	fn new(value: T) -> Self {
		Self {
			obj: value,
			count: 1,
		}
	}
}

impl<T: ?Sized> SharedPtrInner<T> {
	/// Tells whether the inner structure should be dropped.
	fn must_drop(&self) -> bool {
		self.count <= 0
	}
}

/// A shared pointer is a structure which allows to share ownership of a value between several
/// objects. The object counts the number of references to it. When this count reaches zero, the
/// value is freed.
#[derive(Debug)]
pub struct SharedPtr<T: ?Sized> {
	/// A pointer to the inner structure shared by every clones of this structure.
	ptr: NonNull<SharedPtrInner<T>>,
}

impl<T> SharedPtr<T> {
	/// Creates a new shared pointer for the given value `value`.
	pub fn new(value: T) -> Result<SharedPtr::<T>, Errno> {
		let ptr = malloc::alloc(size_of::<SharedPtrInner::<T>>())? as *mut SharedPtrInner<T>;
		unsafe {
			write_ptr(ptr, SharedPtrInner::new(value));
		}

		Ok(Self {
			ptr: NonNull::new(ptr).unwrap(),
		})
	}
}

impl<T: ?Sized> Clone for SharedPtr<T> {
	fn clone(&self) -> Self {
		unsafe {
			&mut *(self.ptr.as_ptr() as *mut SharedPtrInner::<T>)
		}.count += 1;

		SharedPtr {
			ptr: self.ptr,
		}
	}
}

impl<T: ?Sized> AsRef<T> for SharedPtr<T> {
	fn as_ref(&self) -> &T {
		unsafe {
			&(*self.ptr.as_ptr()).obj
		}
	}
}

impl<T: ?Sized> AsMut<T> for SharedPtr<T> {
	fn as_mut(&mut self) -> &mut T {
		unsafe {
			&mut (*self.ptr.as_ptr()).obj
		}
	}
}

impl<T: ?Sized> Deref for SharedPtr<T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		self.as_ref()
	}
}

impl<T: ?Sized> DerefMut for SharedPtr<T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		self.as_mut()
	}
}

impl<T: ?Sized + Unsize<U>, U: ?Sized> CoerceUnsized<SharedPtr<U>> for SharedPtr<T> {}

impl<T: ?Sized + Unsize<U>, U: ?Sized> DispatchFromDyn<SharedPtr<U>> for SharedPtr<T> {}

impl<T: ?Sized> Drop for SharedPtr<T> {
	fn drop(&mut self) {
		unsafe {
			(*self.ptr.as_mut()).count -= 1;
			if self.ptr.as_ref().must_drop() {
				drop_in_place(self.ptr.as_ptr());
				malloc::free(self.ptr.as_ptr() as *mut _);
			}
		}
	}
}

// TODO WeakPtr

#[cfg(test)]
mod test {
	use super::*;

	#[test_case]
	fn shared_ptr0() {
		let b = SharedPtr::new(42 as usize);
		debug_assert_eq!(*b.unwrap(), 42);
	}

	#[test_case]
	fn shared_ptr1() {
		let b = SharedPtr::new(42 as usize).unwrap();
		let b1 = b.clone();
		debug_assert_eq!(*b, 42);
		debug_assert_eq!(*b1, 42);

		drop(b1);

		debug_assert_eq!(*b, 42);
	}

	// TODO More tests
}
