//! This module contains pointer-like structures.

pub mod cow;

use crate::errno::Errno;
use crate::memory::malloc;
use crate::util::lock::Mutex;
use core::marker::Unsize;
use core::mem::size_of;
use core::ops::CoerceUnsized;
use core::ops::Deref;
use core::ops::DispatchFromDyn;
use core::ptr;
use core::ptr::drop_in_place;
use core::ptr::NonNull;

/// Structure holding the number of pointers to a resource.
struct RefCounter {
	/// The number of shared pointers.
	shared_count: usize,
	/// The number of weak pointers.
	weak_count: usize,
}

impl RefCounter {
	/// Tells whether the object can be accessed from a weak pointer.
	fn is_weak_available(&self) -> bool {
		self.shared_count > 0
	}

	/// Tells whether the inner structure must be dropped.
	fn must_drop(&mut self) -> bool {
		self.shared_count <= 0 && self.weak_count <= 0
	}
}

/// Inner structure of the shared pointer.
///
/// The same instance of this structure is shared with every clones of a `SharedPtr` and `WeakPtr`
/// structures.
///
/// This structure holds the number of SharedPtr and WeakPtr holding it.
///
/// Each time the pointer is cloned, the counter is incremented.
///
/// Each time a copy is dropped, the counter is decremented.
///
/// The inner structure and the object wrapped by the shared pointer is dropped at the moment the
/// counter reaches `0`.
struct SharedPtrInner<T: ?Sized, const INT: bool> {
	/// The reference counter.
	ref_counter: Mutex<RefCounter, INT>,

	/// The resource pointed to by the shared pointer.
	obj: Mutex<T, INT>,
}

impl<T, const INT: bool> SharedPtrInner<T, INT> {
	/// Creates a new instance with the given object.
	///
	/// The shared pointer counter is initialized to `1`.
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

/// A shared pointer is a structure which allows to share ownership of an object
/// between several objects.
///
/// The object counts the number of references to it.
///
/// When this count reaches zero, the object is freed.
#[derive(Debug)]
pub struct SharedPtr<T: ?Sized, const INT: bool = true> {
	/// A pointer to the inner structure shared by every clones of this
	/// structure.
	inner: NonNull<SharedPtrInner<T, INT>>,
}

impl<T, const INT: bool> SharedPtr<T, INT> {
	/// Creates a new shared pointer for the given Mutex `obj` containing the
	/// object.
	pub fn new(obj: T) -> Result<Self, Errno> {
		let inner = unsafe {
			malloc::alloc(size_of::<SharedPtrInner<T, INT>>())? as *mut SharedPtrInner<T, INT>
		};
		unsafe {
			// Safe because the pointer is valid
			ptr::write(inner, SharedPtrInner::<T, INT>::new(obj));
		}

		Ok(Self {
			inner: NonNull::new(inner).unwrap(),
		})
	}
}

impl<T: ?Sized, const INT: bool> SharedPtr<T, INT> {
	/// Returns a mutable reference to the inner structure.
	fn get_inner(&self) -> &mut SharedPtrInner<T, INT> {
		unsafe { &mut *(self.inner.as_ptr() as *mut SharedPtrInner<T, INT>) }
	}

	/// Returns an immutable reference to the object.
	pub fn get(&self) -> &Mutex<T, INT> {
		let inner = self.get_inner();
		&inner.obj
	}

	/// Creates a weak pointer for the current shared pointer.
	pub fn new_weak(&self) -> WeakPtr<T, INT> {
		let inner = self.get_inner();
		let mut refs = inner.ref_counter.lock();
		refs.weak_count += 1;

		WeakPtr {
			inner: self.inner,
		}
	}
}

impl<T: ?Sized, const INT: bool> Clone for SharedPtr<T, INT> {
	fn clone(&self) -> Self {
		// Incrementing the number of shared references
		let inner = self.get_inner();
		let mut refs = inner.ref_counter.lock();
		refs.shared_count += 1;

		Self {
			inner: self.inner,
		}
	}
}

impl<T: ?Sized, const INT: bool> AsRef<Mutex<T, INT>> for SharedPtr<T, INT> {
	fn as_ref(&self) -> &Mutex<T, INT> {
		self.get()
	}
}

impl<T: ?Sized, const INT: bool> Deref for SharedPtr<T, INT> {
	type Target = Mutex<T, INT>;

	fn deref(&self) -> &Self::Target {
		self.as_ref()
	}
}

impl<T: ?Sized + Unsize<U>, U: ?Sized, const INT: bool> CoerceUnsized<SharedPtr<U, INT>>
	for SharedPtr<T, INT>
{
}

impl<T: ?Sized + Unsize<U>, U: ?Sized, const INT: bool> DispatchFromDyn<SharedPtr<U, INT>>
	for SharedPtr<T, INT>
{
}

impl<T: ?Sized, const INT: bool> Drop for SharedPtr<T, INT> {
	fn drop(&mut self) {
		let inner = self.get_inner();

		// Decrementing the number of shared references
		{
			let mut refs = inner.ref_counter.lock();
			refs.shared_count -= 1;

			if !refs.must_drop() {
				return;
			}
		}

		// At this point, the object is guaranteed to not be in use because the number
		// of references is 0 and the callee can only get a reference to the mutex,
		// ensuring it is unlocked before dropping the current pointer

		// Dropping inner structure
		unsafe {
			drop_in_place(inner);
			malloc::free(inner as *mut _ as *mut _);
		}
	}
}

/// This type represents a weak pointer except the internal mutex disables
/// interrupts while locked.
pub type IntSharedPtr<T> = SharedPtr<T, false>;

/// A weak pointer is a type of pointer that can be created from a shared
/// pointer.
///
/// It works by keeping a reference to the same object as the shared
/// pointer it was created from.
///
/// However, a weak pointer cannot have the ownership of the object.
///
/// Meaning that when all shared pointers drop the object, the weak pointer shall loose the access
/// to the object.
pub struct WeakPtr<T: ?Sized, const INT: bool = true> {
	/// A pointer to the inner structure shared by every clones of this
	/// structure.
	inner: NonNull<SharedPtrInner<T, INT>>,
}

impl<T: ?Sized, const INT: bool> WeakPtr<T, INT> {
	/// Returns a mutable reference to the inner structure.
	fn get_inner(&self) -> &mut SharedPtrInner<T, INT> {
		unsafe { &mut *(self.inner.as_ptr() as *mut SharedPtrInner<T, INT>) }
	}

	/// Returns an immutable reference to the object.
	pub fn get(&self) -> Option<&Mutex<T, INT>> {
		let inner = self.get_inner();
		let refs = inner.ref_counter.lock();

		if refs.is_weak_available() {
			Some(&inner.obj)
		} else {
			None
		}
	}
}

impl<T: ?Sized, const INT: bool> Clone for WeakPtr<T, INT> {
	fn clone(&self) -> Self {
		// Incrementing the number of weak references
		let inner = self.get_inner();
		let mut refs = inner.ref_counter.lock();
		refs.weak_count += 1;

		Self {
			inner: self.inner,
		}
	}
}

impl<T: ?Sized + Unsize<U>, U: ?Sized, const INT: bool> CoerceUnsized<WeakPtr<U, INT>>
	for WeakPtr<T, INT>
{
}

impl<T: ?Sized + Unsize<U>, U: ?Sized, const INT: bool> DispatchFromDyn<WeakPtr<U, INT>>
	for WeakPtr<T, INT>
{
}

impl<T: ?Sized, const INT: bool> Drop for WeakPtr<T, INT> {
	fn drop(&mut self) {
		let inner = self.get_inner();

		// Decrementing the number of shared references
		{
			let mut refs = inner.ref_counter.lock();
			refs.weak_count -= 1;

			if !refs.must_drop() {
				return;
			}
		}

		// At this point, the object is guaranteed to not be in use because the number
		// of references is 0 and the callee can only get a reference to the mutex,
		// ensuring it is unlocked before dropping the current pointer

		// Dropping inner structure
		unsafe {
			drop_in_place(inner);
			malloc::free(inner as *mut _ as *mut _);
		}
	}
}

/// This type represents a weak pointer except the internal mutex disables
/// interrupts while locked.
pub type IntWeakPtr<T> = WeakPtr<T, false>;
