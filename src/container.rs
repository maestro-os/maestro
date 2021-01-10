/// This files implements containers. These are different from data structures in the fact that
/// they require a memory allocator.

use core::cmp::max;
use core::ffi::c_void;
use core::marker::Unsize;
use core::mem::size_of_val;
use core::mem::transmute;
use core::ops::CoerceUnsized;
use core::ops::DispatchFromDyn;
use core::ops::Index;
use core::ops::IndexMut;
use crate::util;
use mem_alloc::malloc;

/// A vector container is a dynamically-resizable array of elements.
/// When resizing a vector, the elements can be moved, thus the callee should not rely on pointers
/// to elements inside a vector.
pub struct Vec<T> {
	/// The number of elements present in the vector 
	len: usize,
	/// The number of elements that can be stored in the vector with its current buffer 
	capacity: usize,
	/// A pointer to the first element of the vector 
	data: Option<*mut T>,
}

impl<T> Vec<T> {
	/// Creates a new empty vector.
	pub fn new() -> Self {
		Self {
			len: 0,
			capacity: 0,
			data: Some(0 as _),
		}
	}

	// TODO Handle fail
	/// Reallocates the vector's data with the vector's capacity.
	fn realloc(&mut self) {
		let ptr = if self.data.is_some() {
			malloc::realloc(self.data.unwrap() as _, self.capacity).unwrap() as _
		} else {
			malloc::alloc(self.capacity).unwrap() as _
		};
		self.data = Some(ptr);
	}

	// TODO Handle fail
	/// Increases the capacity to at least `min` elements.
	fn increase_capacity(&mut self, min: usize) {
		self.capacity = max(self.capacity, min);
		self.realloc();
	}

	/// Creates a new emoty vector with the given capacity.
	pub fn with_capacity(capacity: usize) -> Self {
		let mut vec = Self::new();
		vec.capacity = capacity;
		vec.realloc();
		vec
	}

	/// Returns the number of elements inside of the vector.
	pub fn len(&self) -> usize {
		self.len
	}

	/// Returns true if the vector contains no elements.
	pub fn is_empty(&self) -> bool {
		self.len == 0
	}

	/// Returns the number of elements that can be stored inside of the vector without needing to
	/// reallocate the memory.
	pub fn capacity(&self) -> usize {
		self.capacity
	}

	/// Returns the first element of the vector.
	pub fn first(&mut self) -> &mut T {
		&mut self[0]
	}

	/// Returns the first element of the vector.
	pub fn last(&mut self) -> &mut T {
		let len = self.len;
		&mut self[len - 1]
	}

	/// Inserts an element at position index within the vector, shifting all elements after it to
	/// the right.
	/*pub fn insert(&mut self, index: usize, element: T) {
		// TODO
	}*/

	/// Removes and returns the element at position index within the vector, shifting all elements
	/// after it to the left.
	/*pub fn remove(&mut self, index: usize) -> T {
		// TODO
	}*/

	// TODO Element access with []

	// TODO reserve
	// TODO resize

	/// Appends an element to the back of a collection.
	pub fn push(&mut self, _value: T) {
		// TODO
	}

	/// Removes the last element from a vector and returns it, or None if it is empty.
	pub fn pop(&mut self) -> Option<T> {
		// TODO
		/*if !self.is_empty() {
			self.len -= 1;
			unsafe { // Pointer arithmetic and dereference of raw pointer
				Some(*self.data.unwrap().offset(self.len as _))
			}
		} else {
			None
		}*/
		None
	}

	// TODO Iterators?

	/// Clears the vector, removing all values.
	fn clear(&mut self) {
		// TODO Call drop on each?

		self.len = 0;
		self.capacity = 0;

		if self.data.is_some() {
			malloc::free(self.data.unwrap() as _);
		} else {
			self.data = None;
		}
	}
}

impl<T> Index<usize> for Vec<T> {
	type Output = T;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
		unsafe { // Dereference of raw pointer
			&*self.data.unwrap().offset(index as _)
		}
    }
}

impl<T> IndexMut<usize> for Vec<T> {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
		unsafe { // Dereference of raw pointer
			&mut *self.data.unwrap().offset(index as _)
		}
    }
}

impl<T> Drop for Vec<T> {
	fn drop(&mut self) {
		self.clear();
	}
}

#[fundamental]
pub struct Box<T: ?Sized> {
	/// Pointer to the allocated memory 
	ptr: *mut T,
}

impl<T> Box<T> {
	/// Creates a new instance and places the given value `value` into it.
	/// If the allocation fails, the function shall return an error.
	pub fn new(value: T) -> Result<Box::<T>, ()> {
		let size = size_of_val(&value);
		let b = Self {
			ptr: unsafe { // Use of transmute
				// TODO Check that conversion from thin to fat pointer works
				transmute::<*mut c_void, *mut T>(malloc::alloc(size)?)
			},
		};
		unsafe { // Call to C function
			util::memcpy(b.ptr as _, &value as *const _ as *const _, size);
		}
		Ok(b)
	}

	/// Returns a reference to the object contained into the Box.
	pub fn unwrap(&mut self) -> &mut T {
		unsafe { // Dereference of raw pointer
			&mut *self.ptr
		}
	}
}

impl<T: Clone> Box<T> {
	/// Clones the Box and its content. The type of the wrapped data must implement the Clone trait.
	/// If the allocation fails, the function shall return an error.
    fn clone(&self) -> Result<Self, ()> {
		let obj = unsafe { // Dereference of raw pointer
			&*self.ptr
		};
		Box::new(obj.clone())
    }
}

impl<T: ?Sized + Unsize<U>, U: ?Sized> CoerceUnsized<Box<U>> for Box<T> {}

impl<T: ?Sized + Unsize<U>, U: ?Sized> DispatchFromDyn<Box<U>> for Box<T> {}

impl<T: ?Sized> Drop for Box<T> {
	fn drop(&mut self) {
		malloc::free(self.ptr as _);
	}
}

// TODO Unit tests
