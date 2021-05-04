/// This module contains pointer-like structures.

use core::marker::Unsize;
use core::mem::size_of;
//use core::mem::size_of_val;
use core::ops::CoerceUnsized;
use core::ops::DispatchFromDyn;
use core::ops::{Deref, DerefMut};
use core::ptr::NonNull;
use core::ptr::drop_in_place;
use crate::errno::Errno;
use crate::memory::malloc;
use crate::util::write_ptr;

// TODO Use a spinlock?

/// Drops the given inner structure if necessary.
fn check_inner_drop<T: ?Sized>(inner: &mut SharedPtrInner<T>) {
	inner.check_obj_drop();

	if inner.must_drop() {
		unsafe {
			drop_in_place(inner);
			malloc::free(inner as *mut _ as *mut _);
		}
	}
}

/// Inner structure of the shared pointer. The same instance of this structure is shared with
/// every clones of a SharedPtr structure. This structure holds the number of SharedPtr holding it.
/// Each time the pointer is cloned, the counter is incremented. Each time a copy is dropped, the
/// counter is decrementer. The inner structure and the object wrapped by the shared pointer is
/// dropped at the moment the counter reaches `0`.
#[derive(Debug)]
struct SharedPtrInner<T: ?Sized> {
	/// The number of shared pointers holding the structure.
	shared_count: usize,
	/// The number of weak pointers holding the structure.
	weak_count: usize,

	/// The object stored by the shared pointer.
	obj: T,
}

impl<T> SharedPtrInner<T> {
	/// Creates a new instance with the given object. The shared pointer counter is initialized to
	/// `1`.
	fn new(value: T) -> Self {
		Self {
			shared_count: 1,
			weak_count: 0,

			obj: value,
		}
	}
}

impl<T: ?Sized> SharedPtrInner<T> {
	/// Tells whether the inner structure must be dropped.
	fn must_drop(&self) -> bool {
		self.shared_count <= 0 && self.weak_count <= 0
	}

	/// Drops the object if necessary.
	fn check_obj_drop(&self) {
		// TODO
		//if self.shared_count <= 0 {
			//drop_in_place(self.obj.as_ptr());
			//malloc::free(self.obj.as_ptr() as *mut _);
		//}
	}
}

/// A shared pointer is a structure which allows to share ownership of a value between several
/// objects. The object counts the number of references to it. When this count reaches zero, the
/// value is freed.
#[derive(Debug)]
pub struct SharedPtr<T: ?Sized> {
	/// A pointer to the inner structure shared by every clones of this structure.
	inner: NonNull<SharedPtrInner<T>>,
}

impl<T> SharedPtr<T> {
	/// Creates a new shared pointer for the given value `value`.
	pub fn new(value: T) -> Result<SharedPtr::<T>, Errno> {
		let inner = malloc::alloc(size_of::<SharedPtrInner<T>>())? as *mut SharedPtrInner<T>;
		unsafe {
			write_ptr(inner, SharedPtrInner::new(value));
		}

		Ok(Self {
			inner: NonNull::new(inner).unwrap(),
		})
	}

	/// Creates a weak pointer for the current shared pointer.
	pub fn new_weak(&self) -> WeakPtr<T> {
		unsafe {
			&mut *(self.inner.as_ptr() as *mut SharedPtrInner::<T>)
		}.weak_count += 1;

		WeakPtr {
			inner: self.inner,
		}
	}
}

impl<T: ?Sized> Clone for SharedPtr<T> {
	fn clone(&self) -> Self {
		unsafe {
			&mut *(self.inner.as_ptr() as *mut SharedPtrInner::<T>)
		}.shared_count += 1;

		SharedPtr {
			inner: self.inner,
		}
	}
}

impl<T: ?Sized> AsRef<T> for SharedPtr<T> {
	fn as_ref(&self) -> &T {
		unsafe {
			&(*self.inner.as_ref()).obj
		}
	}
}

impl<T: ?Sized> AsMut<T> for SharedPtr<T> {
	fn as_mut(&mut self) -> &mut T {
		unsafe {
			&mut (*self.inner.as_mut()).obj
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
			let inner = self.inner.as_mut();
			inner.shared_count -= 1;
			check_inner_drop(inner);
		}
	}
}

/// A weak pointer is a type of pointer that can be created from a shared pointer. It works by
/// keeping a reference to the same object as the shared pointer it was created from. However, a
/// weak pointer cannot have the ownership of the object. Meaning that when all shared pointers
/// drop the object, the weak pointer shall loose the access to the object.
pub struct WeakPtr<T: ?Sized> {
	/// A pointer to the inner structure shared by every clones of this structure.
	inner: NonNull<SharedPtrInner<T>>,
}

impl<T: ?Sized> WeakPtr<T> {
	/// Returns an immutable reference to the object.
	pub fn get(&self) -> Option<&T> {
		let obj = unsafe {
			&self.inner.as_ref().obj
		};
		Some(obj) // TODO
	}

	/// Returns a mutable reference to the object.
	pub fn get_mut(&mut self) -> Option<&mut T> {
		let obj = unsafe {
			&mut self.inner.as_mut().obj
		};
		Some(obj) // TODO
	}
}

impl<T: ?Sized> Clone for WeakPtr<T> {
	fn clone(&self) -> Self {
		unsafe {
			&mut *(self.inner.as_ptr() as *mut SharedPtrInner::<T>)
		}.shared_count += 1;

		WeakPtr {
			inner: self.inner,
		}
	}
}

impl<T: ?Sized> Drop for WeakPtr<T> {
	fn drop(&mut self) {
		unsafe {
			let inner = self.inner.as_mut();
			inner.weak_count -= 1;
			check_inner_drop(inner);
		}
	}
}

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

	#[test_case]
	fn weak_ptr0() {
		let shared = SharedPtr::new(42 as usize).unwrap();
		let weak = shared.new_weak();
		debug_assert_eq!(*shared.as_ref(), 42);
		debug_assert_eq!(weak.get(), Some(&42));

		drop(shared);

		debug_assert!(weak.get().is_none());
	}

	// TODO More tests
}
