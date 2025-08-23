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

//! `Box` holds an object on the heap and handles its memory.

use crate::{__alloc, __dealloc, AllocError, TryClone, errno::AllocResult};
use core::{
	alloc::Layout,
	borrow::{Borrow, BorrowMut},
	fmt,
	marker::Unsize,
	mem,
	mem::ManuallyDrop,
	ops::{CoerceUnsized, Deref, DerefMut, DerefPure, DispatchFromDyn},
	ptr::{Unique, drop_in_place},
};

/// A pointer type that uniquely owns a heap allocation of type `T`.
#[repr(transparent)]
pub struct Box<T: ?Sized>(Unique<T>);

impl<T> Box<T> {
	/// Creates a new instance and places the given value `value` into it.
	///
	/// If the allocation fails, the function shall return an error.
	pub fn new(value: T) -> AllocResult<Box<T>> {
		let layout = Layout::for_value(&value);
		if layout.size() != 0 {
			unsafe {
				let ptr = __alloc(layout)?.cast();
				ptr.write(value);
				Ok(Self(Unique::from_non_null(ptr)))
			}
		} else {
			// Prevent double drop
			mem::forget(value);
			Ok(Self(Unique::dangling()))
		}
	}

	/// Consumes the `Box`, returning the wrapped value.
	pub fn into_inner(self) -> T {
		let layout = Layout::for_value(&*self);
		unsafe {
			let t = self.0.as_ptr().read();
			__dealloc(self.0.as_non_null_ptr().cast(), layout);
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
		Self(Unique::new_unchecked(ptr))
	}

	/// Returns the raw pointer inside the `Box`.
	///
	/// # Safety
	///
	/// It is the caller's responsibility to ensure the memory is freed.
	pub fn into_raw(b: Box<T>) -> *mut T {
		ManuallyDrop::new(b).as_mut_ptr()
	}

	/// Returns a pointer to the data wrapped into the `Box`.
	pub fn as_ptr(&self) -> *const T {
		self.0.as_ptr()
	}

	/// Returns a mutable pointer to the data wrapped into the `Box`.
	pub fn as_mut_ptr(&mut self) -> *mut T {
		self.0.as_ptr()
	}
}

impl<T: ?Sized> AsRef<T> for Box<T> {
	fn as_ref(&self) -> &T {
		unsafe { &*self.0.as_ptr() }
	}
}

impl<T: ?Sized> AsMut<T> for Box<T> {
	fn as_mut(&mut self) -> &mut T {
		unsafe { &mut *self.0.as_ptr() }
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

unsafe impl<T: ?Sized> DerefPure for Box<T> {}

impl<T: TryClone<Error = E>, E: From<AllocError>> TryClone for Box<T> {
	type Error = E;

	fn try_clone(&self) -> Result<Self, Self::Error> {
		let new = self.as_ref().try_clone()?;
		Ok(Box::new(new)?)
	}
}

impl<T: ?Sized + Unsize<U>, U: ?Sized> CoerceUnsized<Box<U>> for Box<T> {}

impl<T: ?Sized + Unsize<U>, U: ?Sized> DispatchFromDyn<Box<U>> for Box<T> {}

impl<T: ?Sized + fmt::Display> fmt::Display for Box<T> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt::Display::fmt(&**self, f)
	}
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for Box<T> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt::Debug::fmt(&**self, f)
	}
}

impl<T: ?Sized> Drop for Box<T> {
	fn drop(&mut self) {
		unsafe {
			let layout = Layout::for_value_raw(self.0.as_ptr());
			if layout.size() != 0 {
				drop_in_place(self.as_mut());
				__dealloc(self.0.as_non_null_ptr().cast(), layout);
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
