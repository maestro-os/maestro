//! This module contains pointer-like structures.

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
use crate::util::lock::Mutex;

/// Drops the given inner structure if necessary.
fn check_inner_drop<T: ?Sized, const INT: bool>(inner: &mut SharedPtrInner<T, INT>) {
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
struct SharedPtrInner<T: ?Sized, const INT: bool> {
	/// The structure storing the pointers count.
	ref_counter: Mutex<RefCounter>,

	/// The object stored by the shared pointer.
	/// When locked, this object requires to disable interruptions because the pointer may contain
	/// sensitive data (for example, it may point to a process. In which case, if an interrupt for
	/// the scheduler happens while cloning the shared pointer, a deadlock may occur).
	obj: Mutex<T, INT>,
}

impl<T, const INT: bool> SharedPtrInner<T, INT> {
	/// Creates a new instance with the given object. The shared pointer counter is initialized to
	/// `1`.
	fn new(obj: T) -> Self {
		Self {
			ref_counter: Mutex::new(RefCounter {
				shared_count: 1,
				weak_count: 0,
			}),

			obj: Mutex::new(obj),
		}
	}
}

impl<T: ?Sized, const INT: bool> SharedPtrInner<T, INT> {
	/// Tells whether the object can be accessed from a weak pointer.
	fn is_weak_available(&mut self) -> bool {
		let guard = self.ref_counter.lock();
		let refs = guard.get();
		refs.shared_count > 0
	}

	/// Tells whether the inner structure must be dropped.
	fn must_drop(&mut self) -> bool {
		let guard = self.ref_counter.lock();
		let refs = guard.get();
		refs.shared_count <= 0 && refs.weak_count <= 0
	}
}

/// A shared pointer is a structure which allows to share ownership of an object between several
/// objects. The object counts the number of references to it. When this count reaches zero, the
/// object is freed.
pub struct SharedPtr<T: ?Sized, const INT: bool = true> {
	/// A pointer to the inner structure shared by every clones of this structure.
	inner: NonNull<SharedPtrInner<T, INT>>,
}

impl<T, const INT: bool> SharedPtr<T, INT> {
	/// Creates a new shared pointer for the given Mutex `obj` containing the object.
	pub fn new(obj: T) -> Result<Self, Errno> {
		let inner = unsafe {
			malloc::alloc(size_of::<SharedPtrInner<T, INT>>())? as *mut SharedPtrInner<T, INT>
		};
		unsafe { // Safe because the pointer is valid
			ptr::write_volatile(inner, SharedPtrInner::<T, INT>::new(obj));
		}

		Ok(Self {
			inner: NonNull::new(inner).unwrap(),
		})
	}
}

impl<T: ?Sized, const INT: bool> SharedPtr<T, INT> {
	/// Returns a mutable reference to the inner structure.
	fn get_inner(&self) -> &mut SharedPtrInner<T, INT> {
		unsafe {
			&mut *(self.inner.as_ptr() as *mut SharedPtrInner<T, INT>)
		}
	}

	/// Returns an immutable reference to the object.
	pub fn get(&self) -> &Mutex<T, INT> {
		let inner = self.get_inner();
		&inner.obj
	}

	/// Returns a mutable reference to the object.
	pub fn get_mut(&self) -> &mut Mutex<T, INT> {
		let inner = self.get_inner();
		&mut inner.obj
	}

	/// Creates a weak pointer for the current shared pointer.
	pub fn new_weak(&self) -> WeakPtr<T, INT> {
		let inner = self.get_inner();
		let mut guard = inner.ref_counter.lock();
		let refs = guard.get_mut();
		refs.weak_count += 1;

		WeakPtr {
			inner: self.inner,
		}
	}
}

impl<T: ?Sized, const INT: bool> Clone for SharedPtr<T, INT> {
	fn clone(&self) -> Self {
		let inner = self.get_inner();
		let mut guard = inner.ref_counter.lock();
		let refs = guard.get_mut();
		refs.shared_count += 1;

		Self {
			inner: self.inner,
		}
	}
}

impl<T: ?Sized, const INT: bool> AsRef<Mutex<T, INT>> for SharedPtr<T, INT> {
	fn as_ref(&self) -> &Mutex<T, INT> {
		unsafe {
			&(*self.inner.as_ref()).obj
		}
	}
}

impl<T: ?Sized, const INT: bool> AsMut<Mutex<T, INT>> for SharedPtr<T, INT> {
	fn as_mut(&mut self) -> &mut Mutex<T, INT> {
		unsafe {
			&mut (*self.inner.as_mut()).obj
		}
	}
}

impl<T: ?Sized, const INT: bool> Deref for SharedPtr<T, INT> {
	type Target = Mutex<T, INT>;

	fn deref(&self) -> &Self::Target {
		self.as_ref()
	}
}

impl<T: ?Sized, const INT: bool> DerefMut for SharedPtr<T, INT> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		self.as_mut()
	}
}

impl<T: ?Sized + Unsize<U>, U: ?Sized, const INT: bool> CoerceUnsized<SharedPtr<U, INT>>
	for SharedPtr<T, INT> {}

impl<T: ?Sized + Unsize<U>, U: ?Sized, const INT: bool> DispatchFromDyn<SharedPtr<U, INT>>
	for SharedPtr<T, INT> {}

impl<T: ?Sized, const INT: bool> Drop for SharedPtr<T, INT> {
	fn drop(&mut self) {
		let inner = self.get_inner();
		{
			let mut guard = inner.ref_counter.lock();
			let refs = guard.get_mut();
			refs.shared_count -= 1;
		}

		check_inner_drop(inner);
	}
}

/// This type represents a weak pointer except the internal mutex disables interrupts while locked.
pub type IntSharedPtr<T> = SharedPtr<T, false>;

/// A weak pointer is a type of pointer that can be created from a shared pointer. It works by
/// keeping a reference to the same object as the shared pointer it was created from. However, a
/// weak pointer cannot have the ownership of the object. Meaning that when all shared pointers
/// drop the object, the weak pointer shall loose the access to the object.
pub struct WeakPtr<T: ?Sized, const INT: bool = true> {
	/// A pointer to the inner structure shared by every clones of this structure.
	inner: NonNull<SharedPtrInner<T, INT>>,
}

impl<T: ?Sized, const INT: bool> WeakPtr<T, INT> {
	/// Returns a mutable reference to the inner structure.
	fn get_inner(&self) -> &mut SharedPtrInner<T, INT> {
		unsafe {
			&mut *(self.inner.as_ptr() as *mut SharedPtrInner<T, INT>)
		}
	}

	/// Returns an immutable reference to the object.
	pub fn get(&self) -> Option<&Mutex<T, INT>> {
		let inner = self.get_inner();
		if inner.is_weak_available() {
			Some(&inner.obj)
		} else {
			None
		}
	}

	/// Returns a mutable reference to the object.
	pub fn get_mut(&self) -> Option<&mut Mutex<T, INT>> {
		let inner = self.get_inner();
		if inner.is_weak_available() {
			Some(&mut inner.obj)
		} else {
			None
		}
	}
}

impl<T: ?Sized, const INT: bool> Clone for WeakPtr<T, INT> {
	fn clone(&self) -> Self {
		let inner = self.get_inner();
		let mut guard = inner.ref_counter.lock();
		let refs = guard.get_mut();
		refs.shared_count += 1;

		Self {
			inner: self.inner,
		}
	}
}

impl<T: ?Sized + Unsize<U>, U: ?Sized, const INT: bool> CoerceUnsized<WeakPtr<U, INT>>
	for WeakPtr<T, INT> {}

impl<T: ?Sized + Unsize<U>, U: ?Sized, const INT: bool> DispatchFromDyn<WeakPtr<U, INT>>
	for WeakPtr<T, INT> {}

impl<T: ?Sized, const INT: bool> Drop for WeakPtr<T, INT> {
	fn drop(&mut self) {
		let inner = self.get_inner();
		{
			let mut guard = inner.ref_counter.lock();
			let refs = guard.get_mut();
			refs.weak_count -= 1;
		}

		check_inner_drop(inner);
	}
}

/// This type represents a weak pointer except the internal mutex disables interrupts while locked.
pub type IntWeakPtr<T> = WeakPtr<T, false>;
