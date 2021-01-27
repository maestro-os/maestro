/// This files implements containers. These are different from data structures in the fact that
/// they require a memory allocator.

use core::cmp::Ordering;
use core::cmp::max;
use core::ffi::c_void;
use core::marker::Unsize;
use core::mem::size_of;
use core::mem::size_of_val;
use core::mem::transmute;
use core::ops::CoerceUnsized;
use core::ops::DispatchFromDyn;
use core::ops::Index;
use core::ops::IndexMut;
use core::ops::{Deref, DerefMut};
use core::ptr;
use crate::memory::malloc;
use crate::util;

/// A vector container is a dynamically-resizable array of elements.
/// When resizing a vector, the elements can be moved, thus the callee should not rely on pointers
/// to elements inside a vector.
/// The implementation of vectors for the kernel cannot follow the implementation of Rust's
/// standard Vec because it must handle properly when a memory allocation fails.
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
			data: None,
		}
	}

	// TODO Handle fail (do not use unwrap)
	/// Reallocates the vector's data with the vector's capacity.
	fn realloc(&mut self) {
		let size = self.capacity * size_of::<T>();
		let ptr = if self.data.is_some() {
			malloc::realloc(self.data.unwrap() as *mut c_void, size).unwrap() as *mut T
		} else {
			malloc::alloc(size).unwrap() as *mut T
		};
		self.data = Some(ptr);
	}

	// TODO Handle fail
	/// Increases the capacity to at least `min` elements.
	fn increase_capacity(&mut self, min: usize) {
		self.capacity = max(self.capacity, min); // TODO Larger allocations than needed to avoid reallocation all the time
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

	/// Triggers a panic after an invalid access to the vector.
	fn vector_panic(&self, index: usize) -> ! {
		panic!("index out of bounds: the len is {} but the index is {}", self.len, index);
	}

	/// Returns the first element of the vector.
	pub fn first(&mut self) -> T {
		if self.is_empty() {
			self.vector_panic(0);
		}

		unsafe { // Pointer arithmetic and dereference of raw pointer
			ptr::read(self.data.unwrap())
		}
	}

	/// Returns the first element of the vector.
	pub fn last(&mut self) -> T {
		if self.is_empty() {
			self.vector_panic(0);
		}

		unsafe { // Pointer arithmetic and dereference of raw pointer
			ptr::read(self.data.unwrap().offset((self.len - 1) as _))
		}
	}

	/// Inserts an element at position index within the vector, shifting all elements after it to
	/// the right.
	pub fn insert(&mut self, index: usize, element: T) {
		if self.capacity < self.len + 1 {
			// TODO Handle allocation error
			self.increase_capacity(self.capacity + 1);
		}
		debug_assert!(self.capacity >= self.len + 1);

		unsafe { // Pointer arithmetic and dereference of raw pointer
			ptr::copy(self.data.unwrap().offset(index as _),
				self.data.unwrap().offset((index + 1) as _),
				self.len - index);
			ptr::write(self.data.unwrap().offset(index as _), element);
		}
		self.len += 1;
	}

	/// Removes and returns the element at position index within the vector, shifting all elements
	/// after it to the left.
	pub fn remove(&mut self, index: usize) -> T {
		if self.is_empty() {
			self.vector_panic(0);
		}

		let v = unsafe { // Pointer arithmetic and dereference of raw pointer
			ptr::read(self.data.unwrap().offset(index as _))
		};

		unsafe { // Pointer arithmetic and dereference of raw pointer
			ptr::copy(self.data.unwrap().offset((index + 1) as _),
				self.data.unwrap().offset(index as _),
				self.len - index - 1);
		}
		self.len -= 1;

		v
	}

	// TODO reserve
	// TODO resize

	/// Appends an element to the back of a collection.
	pub fn push(&mut self, value: T) {
		if self.capacity < self.len + 1 {
			// TODO Handle allocation error
			self.increase_capacity(self.capacity + 1);
		}
		debug_assert!(self.capacity >= self.len + 1);

		unsafe { // Pointer arithmetic and dereference of raw pointer
			ptr::write(self.data.unwrap().offset(self.len as _), value);
		}
		self.len += 1;
	}

	/// Removes the last element from a vector and returns it, or None if it is empty.
	pub fn pop(&mut self) -> Option<T> {
		if !self.is_empty() {
			self.len -= 1;
			unsafe { // Pointer arithmetic and dereference of raw pointer
				Some(ptr::read(self.data.unwrap().offset(self.len as _)))
			}
		} else {
			None
		}
	}

	/// Clears the vector, removing all values.
	fn clear(&mut self) {
		// TODO Call drop on each?

		self.len = 0;
		self.capacity = 0;

		if self.data.is_some() {
			malloc::free(self.data.unwrap() as _);
			self.data = None;
		}
	}
}

impl<T> Index<usize> for Vec<T> {
	type Output = T;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
		if index >= self.len() {
			self.vector_panic(index);
		}

		unsafe { // Dereference of raw pointer
			&*self.data.unwrap().offset(index as _)
		}
    }
}

impl<T> IndexMut<usize> for Vec<T> {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
		if index >= self.len() {
			self.vector_panic(index);
		}

		unsafe { // Dereference of raw pointer
			&mut *self.data.unwrap().offset(index as _)
		}
    }
}

impl<T: Ord> Vec<T> {
	pub fn binary_search(&self, x: &T) -> Result<usize, usize> {
		self.binary_search_by(move | y | {
			if *y < *x {
				Ordering::Less
			} else if *y > *x {
				Ordering::Greater
			} else {
				Ordering::Equal
			}
		})
	}
}

impl<T> Vec<T> {
	pub fn binary_search_by<'a, F>(&'a self, mut f: F) -> Result<usize, usize>
		where F: FnMut(&'a T) -> Ordering {
		if self.is_empty() {
			return Err(0);
		}

		let mut i = self.len() / 2;
		let mut step_size = self.len() / 4;

		while step_size > 0 {
			let ord = f(&self[i]);
			match ord {
				Ordering::Less => {
					i += step_size;
				},
				Ordering::Greater => {
					i -= step_size;
				},
				_ => {
					break;
				},
			}

			step_size /= 2;
		}

		if f(&self[i]) == Ordering::Equal {
			Ok(i)
		} else {
			Err(i)
		}
	}
}

/// An iterator for the Vec structure.
pub struct VecIterator<'a, T> {
	/// The vector to iterate.
	vec: &'a Vec::<T>,
	/// The current index of the iterator.
	index: usize,
}

impl<'a, T> VecIterator<'a, T> {
	/// Creates a vector iterator for the given reference.
	fn new(vec: &'a Vec::<T>) -> Self {
		VecIterator {
			vec: vec,
			index: 0,
		}
	}
}

impl<'a, T> Iterator for VecIterator<'a, T> {
	type Item = &'a T;

	// TODO Implement every functions?

	fn next(&mut self) -> Option<Self::Item> {
		if self.index < self.vec.len() {
			let e = &self.vec[self.index];
			self.index += 1;
			Some(e)
		} else {
			None
		}
	}

	fn count(self) -> usize {
		self.vec.len()
	}
}

impl<'a, T> IntoIterator for &'a Vec<T> {
	type Item = &'a T;
	type IntoIter = VecIterator<'a, T>;

	fn into_iter(self) -> Self::IntoIter {
		VecIterator::new(&self)
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
}

impl<T: ?Sized> AsRef<T> for Box<T> {
    fn as_ref(&self) -> &T {
		unsafe { // Dereference of raw pointer
			&*self.ptr
		}
    }
}

impl<T: ?Sized> AsMut<T> for Box<T> {
    fn as_mut(&mut self) -> &mut T {
		unsafe { // Dereference of raw pointer
			&mut *self.ptr
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

#[cfg(test)]
mod test {
	use super::*;

	#[test_case]
	fn vec_insert_remove0() {
		let mut v = Vec::<usize>::new();
		debug_assert_eq!(v.len(), 0);

		for i in 0..100 {
			v.insert(i, i);
			debug_assert_eq!(v.len(), i + 1);
			debug_assert_eq!(v[i], i);
		}

		for i in (0..100).rev() {
			debug_assert_eq!(v.remove(i), i);
			debug_assert_eq!(v.len(), i);
		}
	}

	// TODO More tests for insert/remove

	// TODO reserve
	// TODO resize

	#[test_case]
	fn vec_push() {
		let mut v = Vec::<usize>::new();
		debug_assert_eq!(v.len(), 0);

		for i in 0..100 {
			v.push(i);
			debug_assert_eq!(v.len(), i + 1);
			debug_assert_eq!(v[i], i);
		}
	}

	#[test_case]
	fn vec_push_clear() {
		let mut v = Vec::<usize>::new();
		debug_assert_eq!(v.len(), 0);

		for i in 0..100 {
			v.push(i);
			debug_assert_eq!(v.len(), i + 1);
			debug_assert_eq!(v[i], i);
		}

		v.clear();
		debug_assert_eq!(v.len(), 0);
	}

	#[test_case]
	fn vec_push_pop() {
		let mut v = Vec::<usize>::new();
		debug_assert_eq!(v.len(), 0);

		for i in 0..100 {
			v.push(i);
			debug_assert_eq!(v.len(), 1);
			debug_assert_eq!(v.first(), i);
			v.pop();
			debug_assert_eq!(v.len(), 0);
		}
	}

	#[test_case]
	fn vec_binary_search0() {
		let v = Vec::<usize>::new();

		if let Err(v) = v.binary_search(&0) {
			assert!(v == 0);
		} else {
			assert!(false);
		}
	}

	#[test_case]
	fn vec_binary_search1() {
		let mut v = Vec::<usize>::new();
		v.push(0);

		if let Ok(v) = v.binary_search(&0) {
			assert!(v == 0);
		} else {
			assert!(false);
		}
	}

	#[test_case]
	fn vec_binary_search2() {
		let mut v = Vec::<usize>::new();
		v.push(1);

		if let Err(v) = v.binary_search(&0) {
			assert!(v == 0);
		} else {
			assert!(false);
		}
	}

	#[test_case]
	fn vec_binary_search3() {
		let mut v = Vec::<usize>::new();
		v.push(1);
		v.push(2);
		v.push(3);

		if let Ok(v) = v.binary_search(&2) {
			assert!(v == 1);
		} else {
			assert!(false);
		}
	}

	#[test_case]
	fn vec_binary_search4() {
		let mut v = Vec::<usize>::new();
		v.push(0);
		v.push(2);
		v.push(4);
		v.push(6);
		v.push(8);

		if let Ok(v) = v.binary_search(&6) {
			assert!(v == 3);
		} else {
			assert!(false);
		}
	}

	#[test_case]
	fn box0() {
		let b = Box::new(42 as usize);
		debug_assert_eq!(*b.unwrap(), 42);
	}

	// TODO
}
