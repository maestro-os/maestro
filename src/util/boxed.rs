//! The Box structure allows to hold an object on the heap and handles its memory properly.

use core::ffi::c_void;
use core::marker::Unsize;
use core::mem::ManuallyDrop;
use core::mem::MaybeUninit;
use core::mem::size_of_val;
use core::mem::transmute;
use core::mem;
use core::ops::CoerceUnsized;
use core::ops::DispatchFromDyn;
use core::ops::{Deref, DerefMut};
use core::ptr::NonNull;
use core::ptr::copy_nonoverlapping;
use core::ptr::drop_in_place;
use crate::errno::Errno;
use crate::memory::malloc;

/// This structure allows to store an object in an allocated region of memory.
/// The object is owned by the Box and will be freed whenever the Box is dropped.
/// The Box uses the `malloc` allocator.
pub struct Box<T: ?Sized> {
	/// Pointer to the allocated memory
	ptr: NonNull<T>,
}

impl<T> Box<T> {
	/// Creates a new instance and places the given value `value` into it.
	/// If the allocation fails, the function shall return an error.
	pub fn new(value: T) -> Result<Box::<T>, Errno> {
		let value_ref = &ManuallyDrop::new(value);

		let size = size_of_val(value_ref);
		let ptr = if size > 0 {
			let ptr = unsafe {
				transmute::<*mut c_void, *mut T>(malloc::alloc(size)?)
			};
			unsafe {
				copy_nonoverlapping(value_ref as *const _ as *const u8, ptr as *mut u8, size);
			}
			NonNull::new(ptr).unwrap()
		} else {
			NonNull::dangling()
		};

		Ok(Self {
			ptr: ptr,
		})
	}

	/// Returns the value owned by the Box, taking its ownership.
	pub fn take(self) -> T {
		unsafe {
			let mut t = MaybeUninit::<T>::uninit();
			copy_nonoverlapping(self.ptr.as_ptr(), t.as_mut_ptr(), 1);
			malloc::free(self.ptr.as_ptr() as _);
			mem::forget(self);
			t.assume_init()
		}
	}
}

impl<T: ?Sized> Box<T> {
	/// Creates a new instance from a raw pointer. The newly created Box takes the ownership of the
	/// pointer.
	pub unsafe fn from_raw(ptr: *mut T) -> Self {
		Self {
			ptr: NonNull::new(ptr).unwrap(),
		}
	}

	/// Returns a pointer to the data wrapped into the Box.
	pub fn as_ptr(&self) -> *const T {
		self.ptr.as_ptr()
	}

	/// Returns a mutable pointer to the data wrapped into the Box.
	pub fn as_mut_ptr(&mut self) -> *mut T {
		self.ptr.as_ptr()
	}
}

impl<T: ?Sized> AsRef<T> for Box<T> {
	fn as_ref(&self) -> &T {
		unsafe {
			&*self.ptr.as_ptr()
		}
	}
}

impl<T: ?Sized> AsMut<T> for Box<T> {
	fn as_mut(&mut self) -> &mut T {
		unsafe {
			&mut *self.ptr.as_ptr()
		}
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

impl<T: ?Sized + Clone> Box<T> {
	/// Clones the Box and its content. The type of the wrapped data must implement the Clone trait.
	/// If the allocation fails, the function shall return an error.
	fn failable_clone(&self) -> Result<Self, Errno> {
		let obj = unsafe {
			&*self.ptr.as_ptr()
		};
		Box::new(obj.clone())
	}
}

impl<T: ?Sized + Unsize<U>, U: ?Sized> CoerceUnsized<Box<U>> for Box<T> {}

impl<T: ?Sized + Unsize<U>, U: ?Sized> DispatchFromDyn<Box<U>> for Box<T> {}

impl<T: ?Sized> Drop for Box<T> {
	fn drop(&mut self) {
		// TODO Clean (condition to check if dangling pointer)
		if self.ptr.as_ptr() as *const c_void as usize > 8 {
			unsafe {
				drop_in_place(self.ptr.as_ptr());
				malloc::free(self.ptr.as_ptr() as _);
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

	// TODO More tests
}
