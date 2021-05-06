/// This module contains pointer-like structures.

use core::marker::PhantomData;
use core::marker::Unsize;
use core::mem::size_of;
use core::ops::CoerceUnsized;
use core::ops::DispatchFromDyn;
use core::ops::{Deref, DerefMut};
use core::ptr::NonNull;
use core::ptr::drop_in_place;
use crate::errno::Errno;
use crate::memory::malloc;
use crate::util::lock::mutex::Mutex;
use crate::util::lock::mutex::MutexGuard;
use crate::util::write_ptr;

/// Drops the given inner structure if necessary.
fn check_inner_drop<T: ?Sized>(inner: &mut SharedPtrInner<T>) {
	if inner.must_drop() {
		unsafe {
			drop_in_place(inner);
			malloc::free(inner as *mut _ as *mut _);
		}
	}
}

/// Structure holding the number of pointers to a resource.
struct RefCounter {
	/// The number of shared pointers.
	shared_count: usize,
	/// The number of weak pointers.
	weak_count: usize,
}

/// Inner structure of the shared pointer. The same instance of this structure is shared with
/// every clones of a SharedPtr structure. This structure holds the number of SharedPtr holding it.
/// Each time the pointer is cloned, the counter is incremented. Each time a copy is dropped, the
/// counter is decrementer. The inner structure and the object wrapped by the shared pointer is
/// dropped at the moment the counter reaches `0`.
struct SharedPtrInner<T: ?Sized> {
	/// The lock to use to modify the pointers count.
	lock: Mutex<RefCounter>,

	/// The object stored by the shared pointer.
	obj: T,
}

impl<T> SharedPtrInner<T> {
	/// Creates a new instance with the given object. The shared pointer counter is initialized to
	/// `1`.
	fn new(value: T) -> Self {
		Self {
			lock: Mutex::new(RefCounter {
				shared_count: 1,
				weak_count: 0,
			}),

			obj: value,
		}
	}
}

impl<T: ?Sized> SharedPtrInner<T> {
	/// Tells whether the object can be accessed from a weak pointer.
	fn is_weak_available(&mut self) -> bool {
		let guard = MutexGuard::new(&mut self.lock);
		let refs = guard.get();
		refs.shared_count > 0
	}

	/// Tells whether the inner structure must be dropped.
	fn must_drop(&mut self) -> bool {
		let guard = MutexGuard::new(&mut self.lock);
		let refs = guard.get();
		refs.shared_count <= 0 && refs.weak_count <= 0
	}
}

/// A shared pointer is a structure which allows to share ownership of a value between several
/// objects. The object counts the number of references to it. When this count reaches zero, the
/// value is freed.
pub struct SharedPtr<T: ?Sized> {
	/// A pointer to the inner structure shared by every clones of this structure.
	inner: NonNull<SharedPtrInner<T>>,

	phantom: PhantomData<SharedPtrInner<T>>,
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

			phantom: PhantomData,
		})
	}
}

impl<T: ?Sized> SharedPtr<T> {
	/// Returns a mutable reference to the inner structure.
	fn get_inner(&self) -> &mut SharedPtrInner<T> {
		unsafe {
			&mut *(self.inner.as_ptr() as *mut SharedPtrInner::<T>)
		}
	}

	/// Creates a weak pointer for the current shared pointer.
	pub fn new_weak(&self) -> WeakPtr<T> {
		let inner = self.get_inner();
		let mut guard = MutexGuard::new(&mut inner.lock);
		let refs = guard.get_mut();
		refs.weak_count += 1;

		WeakPtr {
			inner: self.inner,

			phantom: PhantomData,
		}
	}
}

impl<T: ?Sized> Clone for SharedPtr<T> {
	fn clone(&self) -> Self {
		let inner = self.get_inner();
		let mut guard = MutexGuard::new(&mut inner.lock);
		let refs = guard.get_mut();
		refs.shared_count += 1;

		Self {
			inner: self.inner,

			phantom: PhantomData,
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
		let inner = self.get_inner();
		{
			let mut guard = MutexGuard::new(&mut inner.lock);
			let refs = guard.get_mut();
			refs.shared_count -= 1;
		}

		check_inner_drop(inner);
	}
}

/// A weak pointer is a type of pointer that can be created from a shared pointer. It works by
/// keeping a reference to the same object as the shared pointer it was created from. However, a
/// weak pointer cannot have the ownership of the object. Meaning that when all shared pointers
/// drop the object, the weak pointer shall loose the access to the object.
pub struct WeakPtr<T: ?Sized> {
	/// A pointer to the inner structure shared by every clones of this structure.
	inner: NonNull<SharedPtrInner<T>>,

	phantom: PhantomData<SharedPtrInner<T>>,
}

impl<T: ?Sized> WeakPtr<T> {
	/// Returns a mutable reference to the inner structure.
	fn get_inner(&self) -> &mut SharedPtrInner<T> {
		unsafe {
			&mut *(self.inner.as_ptr() as *mut SharedPtrInner::<T>)
		}
	}

	/// Returns an immutable reference to the object.
	pub fn get(&self) -> Option<&T> {
		let inner = self.get_inner();
		if inner.is_weak_available() {
			Some(&inner.obj)
		} else {
			None
		}
	}

	/// Returns a mutable reference to the object.
	pub fn get_mut(&mut self) -> Option<&mut T> {
		let inner = self.get_inner();
		if inner.is_weak_available() {
			Some(&mut inner.obj)
		} else {
			None
		}
	}
}

impl<T: ?Sized> Clone for WeakPtr<T> {
	fn clone(&self) -> Self {
		let inner = self.get_inner();
		let mut guard = MutexGuard::new(&mut inner.lock);
		let refs = guard.get_mut();
		refs.shared_count += 1;

		Self {
			inner: self.inner,

			phantom: PhantomData,
		}
	}
}

impl<T: ?Sized> Drop for WeakPtr<T> {
	fn drop(&mut self) {
		let inner = self.get_inner();
		{
			let mut guard = MutexGuard::new(&mut inner.lock);
			let refs = guard.get_mut();
			refs.weak_count -= 1;
		}

		check_inner_drop(inner);
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
