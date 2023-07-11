//! The `Box` structure allows to hold an object on the heap and handles its
//! memory properly.

use crate::errno::Errno;
use crate::memory;
use crate::memory::malloc;
use crate::util::EResult;
use crate::util::TryClone;
use crate::util::TryDefault;
use core::borrow::{Borrow, BorrowMut};
use core::ffi::c_void;
use core::fmt;
use core::marker::Unsize;
use core::mem;
use core::mem::size_of_val;
use core::ops::CoerceUnsized;
use core::ops::DispatchFromDyn;
use core::ops::{Deref, DerefMut};
use core::ptr;
use core::ptr::drop_in_place;
use core::ptr::NonNull;

/// A `Box` allows to store an object on the heap.
///
/// The object is owned by the Box and will be freed whenever it is dropped.
///
/// Box uses the `malloc` allocator.
pub struct Box<T: ?Sized> {
	/// Pointer to the allocated memory
	ptr: NonNull<T>,
}

impl<T: TryDefault> TryDefault for Box<T> {
	fn try_default() -> EResult<Self> {
		Self::new(T::try_default()?)
	}
}

impl<T> Box<T> {
	/// Creates a new instance and places the given value `value` into it.
	///
	/// If the allocation fails, the function shall return an error.
	pub fn new(value: T) -> Result<Box<T>, Errno> {
		let ptr = {
			let size = size_of_val(&value);

			if size > 0 {
				let ptr = unsafe { malloc::alloc(size)? as *mut T };
				unsafe {
					ptr::copy_nonoverlapping(
						&value as *const _ as *const u8,
						ptr as *mut u8,
						size,
					);
				}

				NonNull::new(ptr).unwrap()
			} else {
				NonNull::dangling()
			}
		};

		// Prevent double drop
		mem::forget(value);

		Ok(Self {
			ptr,
		})
	}

	/// Returns the value owned by the `Box`, taking its ownership.
	pub fn take(self) -> T {
		unsafe {
			let t = ptr::read(self.ptr.as_ptr());

			malloc::free(self.ptr.as_ptr() as _);
			mem::forget(self);

			t
		}
	}
}

impl<T: ?Sized> Box<T> {
	/// Creates a new instance from a raw pointer.
	///
	/// The newly created `Box` takes the ownership of the pointer.
	///
	/// The given pointer must be an address to a region of memory allocated
	/// with the memory allocator since its the allocator that the `Box` will use
	/// to free it.
	pub unsafe fn from_raw(ptr: *mut T) -> Self {
		Self {
			ptr: NonNull::new(ptr).unwrap(),
		}
	}

	/// Returns a pointer to the data wrapped into the `Box`.
	pub fn as_ptr(&self) -> *const T {
		self.ptr.as_ptr()
	}

	/// Returns a mutable pointer to the data wrapped into the `Box`.
	pub fn as_mut_ptr(&mut self) -> *mut T {
		self.ptr.as_ptr()
	}
}

impl<T: ?Sized> AsRef<T> for Box<T> {
	fn as_ref(&self) -> &T {
		unsafe { &*self.ptr.as_ptr() }
	}
}

impl<T: ?Sized> AsMut<T> for Box<T> {
	fn as_mut(&mut self) -> &mut T {
		unsafe { &mut *self.ptr.as_ptr() }
	}
}

impl<T: ?Sized> Borrow<T> for Box<T> {
	fn borrow(&self) -> &T {
		self.as_ref()
	}
}

impl<T: ?Sized> BorrowMut<T> for Box<T> {
	fn borrow_mut(&mut self) -> &mut T {
		self.as_mut()
	}
}

impl<T: ?Sized> Deref for Box<T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		self.as_ref()
	}
}

impl<T: ?Sized> DerefMut for Box<T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		self.as_mut()
	}
}

impl<T: ?Sized + Clone> TryClone for Box<T> {
	fn try_clone(&self) -> Result<Self, Errno> {
		let obj = unsafe { &*self.ptr.as_ptr() };

		Box::new(obj.clone())
	}
}

impl<T: ?Sized + Unsize<U>, U: ?Sized> CoerceUnsized<Box<U>> for Box<T> {}

impl<T: ?Sized + Unsize<U>, U: ?Sized> DispatchFromDyn<Box<U>> for Box<T> {}

impl<T: ?Sized + fmt::Display> fmt::Display for Box<T> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.as_ref())
	}
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for Box<T> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{:?}", self.as_ref())
	}
}

impl<T: ?Sized> Drop for Box<T> {
	fn drop(&mut self) {
		let ptr = self.ptr.as_ptr();

		if (ptr as *const c_void as usize) >= memory::PAGE_SIZE {
			unsafe {
				drop_in_place(ptr);
				malloc::free(ptr as _);
			}
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test_case]
	fn box0() {
		let b = Box::new(42 as usize);
		debug_assert_eq!(*b.unwrap(), 42);
	}
}
