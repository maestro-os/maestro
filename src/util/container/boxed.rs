/// TODO doc

use core::ffi::c_void;
use core::marker::Unsize;
use core::mem::size_of_val;
use core::mem::transmute;
use core::ops::CoerceUnsized;
use core::ops::DispatchFromDyn;
use core::ops::{Deref, DerefMut};
use core::ptr::NonNull;
use core::ptr::copy_nonoverlapping;
use crate::memory::malloc;

/// TODO doc
#[fundamental]
pub struct Box<T: ?Sized> {
	/// Pointer to the allocated memory
	ptr: NonNull<T>,
}

impl<T> Box<T> {
	/// Creates a new instance and places the given value `value` into it.
	/// If the allocation fails, the function shall return an error.
	pub fn new(value: T) -> Result<Box::<T>, ()> {
		let size = size_of_val(&value);
		let ptr = if size > 0 {
			let ptr = unsafe { // Use of transmute
				// TODO Check that conversion from thin to fat pointer works
				transmute::<*mut c_void, *mut T>(malloc::alloc(size)?)
			};
			unsafe { // Call to unsafe function
				copy_nonoverlapping(&value as *const _ as *const _, ptr as _, size);
			}
			NonNull::new(ptr).unwrap()
		} else {
			NonNull::dangling()
		};
		Ok(Self {
			ptr: ptr,
		})
	}
}

impl<T: ?Sized> AsRef<T> for Box<T> {
    fn as_ref(&self) -> &T {
		unsafe { // Dereference of raw pointer
			&*self.ptr.as_ptr()
		}
    }
}

impl<T: ?Sized> AsMut<T> for Box<T> {
    fn as_mut(&mut self) -> &mut T {
		unsafe { // Dereference of raw pointer
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

impl<T: Clone> Box<T> {
	/// Clones the Box and its content. The type of the wrapped data must implement the Clone trait.
	/// If the allocation fails, the function shall return an error.
    fn clone(&self) -> Result<Self, ()> {
		let obj = unsafe { // Dereference of raw pointer
			&*self.ptr.as_ptr()
		};
		Box::new(obj.clone())
    }
}

impl<T: ?Sized + Unsize<U>, U: ?Sized> CoerceUnsized<Box<U>> for Box<T> {}

impl<T: ?Sized + Unsize<U>, U: ?Sized> DispatchFromDyn<Box<U>> for Box<T> {}

impl<T: ?Sized> Drop for Box<T> {
	fn drop(&mut self) {
		malloc::free(self.ptr.as_ptr() as _);
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

	// TODO
}
