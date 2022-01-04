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
use crate::util::lock::DummyIntManager;
use crate::util::lock::IntManager;
use crate::util::lock::Mutex;
use crate::util::lock::NormalIntManager;

/// Drops the given inner structure if necessary.
fn check_inner_drop<T: ?Sized, I: IntManager>(inner: &mut SharedPtrInner<T, I>) {
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
struct SharedPtrInner<T: ?Sized, I: IntManager> {
	/// The structure storing the pointers count.
	ref_counter: Mutex<RefCounter>,

	/// The object stored by the shared pointer.
	/// When locked, this object requires to disable interruptions because the pointer may contain
	/// sensitive data (for example, it may point to a process. In which case, if an interrupt for
	/// the scheduler happens while cloning the shared pointer, a deadlock may occur).
	obj: Mutex<T, I>,
}

impl<T, I: IntManager> SharedPtrInner<T, I> {
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

impl<T: ?Sized, I: IntManager> SharedPtrInner<T, I> {
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
pub struct SharedPtr<T: ?Sized, I: IntManager = DummyIntManager> {
	/// A pointer to the inner structure shared by every clones of this structure.
	inner: NonNull<SharedPtrInner<T, I>>,
}

impl<T, I: IntManager> SharedPtr<T, I> {
	/// Creates a new shared pointer for the given Mutex `obj` containing the object.
	pub fn new(obj: T) -> Result<Self, Errno> {
		let inner = unsafe {
			malloc::alloc(size_of::<SharedPtrInner<T, I>>())? as *mut SharedPtrInner<T, I>
		};
		unsafe { // Safe because the pointer is valid
			ptr::write_volatile(inner, SharedPtrInner::<T, I>::new(obj));
		}

		Ok(Self {
			inner: NonNull::new(inner).unwrap(),
		})
	}
}

impl<T: ?Sized, I: IntManager> SharedPtr<T, I> {
	/// Returns a mutable reference to the inner structure.
	fn get_inner(&self) -> &mut SharedPtrInner<T, I> {
		unsafe {
			&mut *(self.inner.as_ptr() as *mut SharedPtrInner<T, I>)
		}
	}

	/// Returns an immutable reference to the object.
	pub fn get(&self) -> &Mutex<T, I> {
		let inner = self.get_inner();
		&inner.obj
	}

	/// Returns a mutable reference to the object.
	pub fn get_mut(&self) -> &mut Mutex<T, I> {
		let inner = self.get_inner();
		&mut inner.obj
	}

	/// Creates a weak pointer for the current shared pointer.
	pub fn new_weak(&self) -> WeakPtr<T, I> {
		let inner = self.get_inner();
		let mut guard = inner.ref_counter.lock();
		let refs = guard.get_mut();
		refs.weak_count += 1;

		WeakPtr {
			inner: self.inner,
		}
	}
}

impl<T: ?Sized, I: IntManager> Clone for SharedPtr<T, I> {
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

impl<T: ?Sized, I: IntManager> AsRef<Mutex<T, I>> for SharedPtr<T, I> {
	fn as_ref(&self) -> &Mutex<T, I> {
		unsafe {
			&(*self.inner.as_ref()).obj
		}
	}
}

impl<T: ?Sized, I: IntManager> AsMut<Mutex<T, I>> for SharedPtr<T, I> {
	fn as_mut(&mut self) -> &mut Mutex<T, I> {
		unsafe {
			&mut (*self.inner.as_mut()).obj
		}
	}
}

impl<T: ?Sized, I: IntManager> Deref for SharedPtr<T, I> {
	type Target = Mutex<T, I>;

	fn deref(&self) -> &Self::Target {
		self.as_ref()
	}
}

impl<T: ?Sized, I: IntManager> DerefMut for SharedPtr<T, I> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		self.as_mut()
	}
}

impl<T: ?Sized + Unsize<U>, U: ?Sized, I: IntManager> CoerceUnsized<SharedPtr<U, I>>
	for SharedPtr<T, I> {}

impl<T: ?Sized + Unsize<U>, U: ?Sized, I: IntManager> DispatchFromDyn<SharedPtr<U, I>>
	for SharedPtr<T, I> {}

impl<T: ?Sized, I: IntManager> Drop for SharedPtr<T, I> {
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
pub type IntSharedPtr<T> = SharedPtr<T, NormalIntManager>;

/// A weak pointer is a type of pointer that can be created from a shared pointer. It works by
/// keeping a reference to the same object as the shared pointer it was created from. However, a
/// weak pointer cannot have the ownership of the object. Meaning that when all shared pointers
/// drop the object, the weak pointer shall loose the access to the object.
pub struct WeakPtr<T: ?Sized, I: IntManager = DummyIntManager> {
	/// A pointer to the inner structure shared by every clones of this structure.
	inner: NonNull<SharedPtrInner<T, I>>,
}

impl<T: ?Sized, I: IntManager> WeakPtr<T, I> {
	/// Returns a mutable reference to the inner structure.
	fn get_inner(&self) -> &mut SharedPtrInner<T, I> {
		unsafe {
			&mut *(self.inner.as_ptr() as *mut SharedPtrInner<T, I>)
		}
	}

	/// Returns an immutable reference to the object.
	pub fn get(&self) -> Option<&Mutex<T, I>> {
		let inner = self.get_inner();
		if inner.is_weak_available() {
			Some(&inner.obj)
		} else {
			None
		}
	}

	/// Returns a mutable reference to the object.
	pub fn get_mut(&self) -> Option<&mut Mutex<T, I>> {
		let inner = self.get_inner();
		if inner.is_weak_available() {
			Some(&mut inner.obj)
		} else {
			None
		}
	}
}

impl<T: ?Sized, I: IntManager> Clone for WeakPtr<T, I> {
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

impl<T: ?Sized + Unsize<U>, U: ?Sized, I: IntManager> CoerceUnsized<WeakPtr<U, I>>
	for WeakPtr<T, I> {}

impl<T: ?Sized + Unsize<U>, U: ?Sized, I: IntManager> DispatchFromDyn<WeakPtr<U, I>>
	for WeakPtr<T, I> {}

impl<T: ?Sized, I: IntManager> Drop for WeakPtr<T, I> {
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
pub type IntWeakPtr<T> = WeakPtr<T, NormalIntManager>;
