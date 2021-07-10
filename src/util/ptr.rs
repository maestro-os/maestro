//! This module contains pointer-like structures.

use core::marker::PhantomData;
use core::marker::Unsize;
use core::mem::size_of;
use core::ops::CoerceUnsized;
use core::ops::DispatchFromDyn;
use core::ops::{Deref, DerefMut};
use core::ptr::NonNull;
use core::ptr::drop_in_place;
use core::ptr;
use crate::errno::Errno;
use crate::memory::malloc;
use crate::util::lock::mutex::Mutex;
use crate::util::lock::mutex::MutexGuard;
use crate::util::lock::mutex::TMutex;

/// Drops the given inner structure if necessary.
fn check_inner_drop<T: ?Sized, M: TMutex<T> + ?Sized>(inner: &mut SharedPtrInner<M>) {
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
	/// The structure storing the pointers count.
	ref_counter: Mutex<RefCounter>,

	/// The object stored by the shared pointer.
	obj: T,
}

impl<T> SharedPtrInner<T> {
	/// Creates a new instance with the given object. The shared pointer counter is initialized to
	/// `1`.
	fn new(obj: T) -> Self {
		Self {
			ref_counter: Mutex::new(RefCounter {
				shared_count: 1,
				weak_count: 0,
			}),

			obj,
		}
	}
}

impl<T: ?Sized> SharedPtrInner<T> {
	/// Tells whether the object can be accessed from a weak pointer.
	fn is_weak_available(&mut self) -> bool {
		let guard = MutexGuard::new(&mut self.ref_counter);
		let refs = guard.get();
		refs.shared_count > 0
	}

	/// Tells whether the inner structure must be dropped.
	fn must_drop(&mut self) -> bool {
		let guard = MutexGuard::new(&mut self.ref_counter);
		let refs = guard.get();
		refs.shared_count <= 0 && refs.weak_count <= 0
	}
}

/// A shared pointer is a structure which allows to share ownership of an object between several
/// objects. The object counts the number of references to it. When this count reaches zero, the
/// object is freed.
pub struct SharedPtr<T: ?Sized, M: TMutex<T> + ?Sized = Mutex<T>> {
	/// A pointer to the inner structure shared by every clones of this structure.
	inner: NonNull<SharedPtrInner<M>>,

	phantom: PhantomData<T>,
}

impl<T, M: TMutex<T>> SharedPtr<T, M> {
	/// Creates a new shared pointer for the given Mutex `obj` containing the object.
	pub fn new(obj: M) -> Result<Self, Errno> {
		let inner = unsafe {
			malloc::alloc(size_of::<SharedPtrInner<M>>())? as *mut SharedPtrInner<M>
		};
		unsafe { // Safe because the pointer is valid
			ptr::write_volatile(inner, SharedPtrInner::new(obj));
		}

		Ok(Self {
			inner: NonNull::new(inner).unwrap(),

			phantom: PhantomData,
		})
	}
}

impl<T: ?Sized, M: TMutex<T> + ?Sized> SharedPtr<T, M> {
	/// Returns a mutable reference to the inner structure.
	fn get_inner(&self) -> &mut SharedPtrInner<M> {
		unsafe {
			&mut *(self.inner.as_ptr() as *mut SharedPtrInner<M>)
		}
	}

	/// Returns an immutable reference to the object.
	pub fn get(&self) -> &M {
		let inner = self.get_inner();
		&inner.obj
	}

	/// Returns a mutable reference to the object.
	pub fn get_mut(&self) -> &mut M {
		let inner = self.get_inner();
		&mut inner.obj
	}

	/// Creates a weak pointer for the current shared pointer.
	pub fn new_weak(&self) -> WeakPtr<T, M> {
		let inner = self.get_inner();
		let mut guard = MutexGuard::new(&mut inner.ref_counter);
		let refs = guard.get_mut();
		refs.weak_count += 1;

		WeakPtr {
			inner: self.inner,

			phantom: PhantomData,
		}
	}
}

impl<T: ?Sized, M: TMutex<T> + ?Sized> Clone for SharedPtr<T, M> {
	fn clone(&self) -> Self {
		let inner = self.get_inner();
		let mut guard = MutexGuard::new(&mut inner.ref_counter);
		let refs = guard.get_mut();
		refs.shared_count += 1;

		Self {
			inner: self.inner,

			phantom: PhantomData,
		}
	}
}

impl<T: ?Sized, M: TMutex<T> + ?Sized> AsRef<M> for SharedPtr<T, M> {
	fn as_ref(&self) -> &M {
		unsafe {
			&(*self.inner.as_ref()).obj
		}
	}
}

impl<T: ?Sized, M: TMutex<T> + ?Sized> AsMut<M> for SharedPtr<T, M> {
	fn as_mut(&mut self) -> &mut M {
		unsafe {
			&mut (*self.inner.as_mut()).obj
		}
	}
}

impl<T: ?Sized, M: TMutex<T> + ?Sized> Deref for SharedPtr<T, M> {
	type Target = M;

	fn deref(&self) -> &Self::Target {
		self.as_ref()
	}
}

impl<T: ?Sized, M: TMutex<T> + ?Sized> DerefMut for SharedPtr<T, M> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		self.as_mut()
	}
}

impl<T: ?Sized + Unsize<U>, U: ?Sized, M: TMutex<T> + ?Sized + Unsize<MU>, MU: TMutex<U> + ?Sized>
	CoerceUnsized<SharedPtr<U, MU>> for SharedPtr<T, M> {}

impl<T: ?Sized + Unsize<U>, U: ?Sized, M: TMutex<T> + ?Sized + Unsize<MU>, MU: TMutex<U> + ?Sized>
	DispatchFromDyn<SharedPtr<U, MU>> for SharedPtr<T, M> {}

impl<T: ?Sized, M: TMutex<T> + ?Sized> Drop for SharedPtr<T, M> {
	fn drop(&mut self) {
		let inner = self.get_inner();
		{
			let mut guard = MutexGuard::new(&mut inner.ref_counter);
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
pub struct WeakPtr<T: ?Sized, M: TMutex<T> + ?Sized = Mutex<T>> {
	/// A pointer to the inner structure shared by every clones of this structure.
	inner: NonNull<SharedPtrInner<M>>,

	phantom: PhantomData<T>,
}

impl<T: ?Sized, M: TMutex<T> + ?Sized> WeakPtr<T, M> {
	/// Returns a mutable reference to the inner structure.
	fn get_inner(&self) -> &mut SharedPtrInner<M> {
		unsafe {
			&mut *(self.inner.as_ptr() as *mut SharedPtrInner<M>)
		}
	}

	/// Returns an immutable reference to the object.
	pub fn get(&self) -> Option<&M> {
		let inner = self.get_inner();
		if inner.is_weak_available() {
			Some(&inner.obj)
		} else {
			None
		}
	}

	/// Returns a mutable reference to the object.
	pub fn get_mut(&self) -> Option<&mut M> {
		let inner = self.get_inner();
		if inner.is_weak_available() {
			Some(&mut inner.obj)
		} else {
			None
		}
	}
}

impl<T: ?Sized, M: TMutex<T> + ?Sized> Clone for WeakPtr<T, M> {
	fn clone(&self) -> Self {
		let inner = self.get_inner();
		let mut guard = MutexGuard::new(&mut inner.ref_counter);
		let refs = guard.get_mut();
		refs.shared_count += 1;

		Self {
			inner: self.inner,

			phantom: PhantomData,
		}
	}
}

impl<T: ?Sized + Unsize<U>, U: ?Sized, M: TMutex<T> + ?Sized + Unsize<MU>, MU: TMutex<U> + ?Sized>
	CoerceUnsized<WeakPtr<U, MU>> for WeakPtr<T, M> {}

impl<T: ?Sized + Unsize<U>, U: ?Sized, M: TMutex<T> + ?Sized + Unsize<MU>, MU: TMutex<U> + ?Sized>
	DispatchFromDyn<WeakPtr<U, MU>> for WeakPtr<T, M> {}

impl<T: ?Sized, M: TMutex<T> + ?Sized> Drop for WeakPtr<T, M> {
	fn drop(&mut self) {
		let inner = self.get_inner();
		{
			let mut guard = MutexGuard::new(&mut inner.ref_counter);
			let refs = guard.get_mut();
			refs.weak_count -= 1;
		}

		check_inner_drop(inner);
	}
}
