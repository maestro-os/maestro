/*
 * Copyright 2024 Luc Lenôtre
 *
 * This file is part of Maestro.
 *
 * Maestro is free software: you can redistribute it and/or modify it under the
 * terms of the GNU General Public License as published by the Free Software
 * Foundation, either version 3 of the License, or (at your option) any later
 * version.
 *
 * Maestro is distributed in the hope that it will be useful, but WITHOUT ANY
 * WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR
 * A PARTICULAR PURPOSE. See the GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along with
 * Maestro. If not, see <https://www.gnu.org/licenses/>.
 */

//! The `Box` structure allows to hold an object on the heap and handles its
//! memory properly.

use crate::{errno::AllocResult, AllocError, TryClone, TryDefault};
use alloc::alloc::Global;
use core::{
	alloc::{Allocator, Layout},
	borrow::{Borrow, BorrowMut},
	fmt,
	marker::Unsize,
	mem,
	mem::ManuallyDrop,
	ops::{CoerceUnsized, Deref, DerefMut, DispatchFromDyn},
	ptr::{drop_in_place, NonNull},
};

/// A `Box` allows to store an object on the heap.
///
/// The object is owned by the Box and will be freed whenever it is dropped.
///
/// Box uses the `malloc` allocator.
pub struct Box<T: ?Sized> {
	/// Pointer to the allocated memory
	ptr: NonNull<T>,
}

impl<T: TryDefault<Error = E>, E: From<AllocError>> TryDefault for Box<T> {
	type Error = E;

	fn try_default() -> Result<Self, Self::Error> {
		Ok(Self::new(T::try_default()?)?)
	}
}

impl<T> Box<T> {
	/// Creates a new instance and places the given value `value` into it.
	///
	/// If the allocation fails, the function shall return an error.
	pub fn new(value: T) -> AllocResult<Box<T>> {
		let layout = Layout::for_value(&value);
		let ptr = if layout.size() > 0 {
			let ptr = Global.allocate(layout)?.cast();
			unsafe {
				ptr.write(value);
			}
			ptr
		} else {
			// Prevent double drop
			mem::forget(value);
			NonNull::dangling()
		};
		Ok(Self {
			ptr,
		})
	}

	/// Returns the value owned by the `Box`, taking its ownership.
	pub fn take(self) -> T {
		let layout = Layout::for_value(&*self);
		unsafe {
			let t = self.ptr.read();
			Global.deallocate(self.ptr.cast(), layout);
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
	/// # Safety
	///
	/// The given pointer must be valid and must point to an address to a region of memory
	/// allocated with the memory allocator since `Box` will use the allocator to free it.
	pub unsafe fn from_raw(ptr: *mut T) -> Self {
		Self {
			ptr: NonNull::new(ptr).unwrap(),
		}
	}

	/// Returns the raw pointer inside of the `Box`.
	///
	/// # Safety
	///
	/// It is the caller's responsibility to ensure the memory is freed.
	pub unsafe fn into_raw(b: Box<T>) -> *mut T {
		ManuallyDrop::new(b).as_mut_ptr()
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

impl<T: TryClone<Error = E>, E: From<AllocError>> TryClone for Box<T> {
	type Error = E;

	fn try_clone(&self) -> Result<Self, Self::Error> {
		let obj = unsafe { &*self.ptr.as_ptr() };
		Ok(Box::new(obj.try_clone()?)?)
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
		let ptr = self.ptr.cast::<()>().as_ptr();
		// If the pointer is not dangling
		if (ptr as usize) >= 4096 {
			unsafe {
				let inner = self.ptr.as_mut();
				let layout = Layout::for_value(inner);
				drop_in_place(inner);
				Global.deallocate(self.ptr.cast(), layout);
			}
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn box0() {
		let b = Box::new(42).unwrap();
		assert_eq!(*b, 42);
	}
}
