/// This module contains pointer-like structures.

use core::ffi::c_void;
use core::marker::Unsize;
use core::mem::ManuallyDrop;
use core::mem::size_of_val;
use core::mem::transmute;
use core::ops::CoerceUnsized;
use core::ops::{Deref, DerefMut};
use core::ptr::NonNull;
use core::ptr::copy_nonoverlapping;
use core::ptr::drop_in_place;
use crate::memory::malloc;
use crate::util::data_struct::ListNode;

/// A shared pointer is a structure which allows to share ownership of a value between several
/// objects. The object counts the number of references to it. When this count reaches zero, the
/// value is freed.
#[derive(Debug)]
pub struct SharedPtr<T: ?Sized> {
	/// The list storing other shared pointers pointing to the same data.
	list: ListNode,
	/// A pointer to the data.
	ptr: NonNull<T>,
}

impl<T> SharedPtr<T> {
	/// Creates a new shared pointer for the given value `value`.
	pub fn new(value: T) -> Result<SharedPtr::<T>, ()> {
		let value_ref = &ManuallyDrop::new(value);

		let size = size_of_val(value_ref);
		let ptr = if size > 0 {
			let ptr = unsafe { // Use of transmute
				// TODO Check that conversion from thin to fat pointer works
				transmute::<*mut c_void, *mut T>(malloc::alloc(size)?)
			};
			unsafe { // Call to unsafe function
				copy_nonoverlapping(value_ref as *const _ as *const u8, ptr as *mut u8, size);
			}
			NonNull::new(ptr).unwrap()
		} else {
			NonNull::dangling()
		};

		Ok(Self {
			list: ListNode::new_single(),
			ptr: ptr,
		})
	}

	/// Clones the shared pointer, sharing the ownership.
	pub fn clone(&mut self) -> Self {
		let mut list = ListNode::new_single();
		list.insert_after(&mut self.list);

		SharedPtr {
			list: list,
			ptr: self.ptr,
		}
	}
}

impl<T: ?Sized> AsRef<T> for SharedPtr<T> {
	fn as_ref(&self) -> &T {
		unsafe { // Dereference of raw pointer
			&*self.ptr.as_ptr()
		}
	}
}

impl<T: ?Sized> AsMut<T> for SharedPtr<T> {
	fn as_mut(&mut self) -> &mut T {
		unsafe { // Dereference of raw pointer
			&mut *self.ptr.as_ptr()
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

impl<T: ?Sized> Drop for SharedPtr<T> {
	fn drop(&mut self) {
		if self.list.is_single() {
			unsafe { // Call to unsafe function
				drop_in_place(self.ptr.as_ptr());
			}
			malloc::free(self.ptr.as_ptr() as *mut _);
		} else {
			self.list.unlink_floating();
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
		let mut b = SharedPtr::new(42 as usize).unwrap();
		let b1 = b.clone();
		debug_assert_eq!(*b, 42);
		debug_assert_eq!(*b1, 42);

		drop(b1);

		debug_assert_eq!(*b, 42);
	}

	// TODO
}
