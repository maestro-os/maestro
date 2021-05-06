/// This module implements the Vec container.

use core::cmp::Ordering;
use core::cmp::max;
use core::ffi::c_void;
use core::mem::size_of;
use core::ops::Index;
use core::ops::IndexMut;
use core::ptr::NonNull;
use core::ptr;
use core::slice;
use crate::errno::Errno;
use crate::memory::malloc;
use crate::util::FailableClone;

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
	data: Option<NonNull<T>>,
}

impl<T> Vec<T> {
	/// Creates a new empty vector.
	pub const fn new() -> Self {
		Self {
			len: 0,
			capacity: 0,
			data: None,
		}
	}

	// TODO Handle fail (do not use unwrap)
	/// Reallocates the vector's data with the vector's capacity.
	fn realloc(&mut self) -> Result<(), Errno> {
		let size = self.capacity * size_of::<T>();
		let ptr = if self.data.is_some() {
			malloc::realloc(self.data.unwrap().as_ptr() as *mut c_void, size)? as *mut T
		} else {
			malloc::alloc(size)? as *mut T
		};
		self.data = NonNull::new(ptr);
		debug_assert!(self.data.is_some());
		Ok(())
	}

	// TODO Handle fail
	/// Increases the capacity to at least `min` elements.
	fn increase_capacity(&mut self, min: usize) -> Result<(), Errno> {
		self.capacity = max(self.capacity, min); // TODO Larger allocations than needed to avoid
		// reallocation all the time
		self.realloc()
	}

	/// Creates a new emoty vector with the given capacity.
	pub fn with_capacity(capacity: usize) -> Result<Self, Errno> {
		let mut vec = Self::new();
		vec.capacity = capacity;
		vec.realloc()?;
		Ok(vec)
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

	/// Returns a slice containing the data.
	pub fn as_slice(&self) -> &[T] {
		unsafe {
			if let Some(p) = self.data {
				slice::from_raw_parts(p.as_ptr(), self.len)
			} else {
				slice::from_raw_parts(NonNull::dangling().as_ptr(), 0)
			}
		}
	}

	/// Returns a mutable slice containing the data.
	pub fn as_mut_slice(&mut self) -> &mut [T] {
		unsafe {
			if let Some(p) = self.data {
				slice::from_raw_parts_mut(p.as_ptr() as *mut T, self.len)
			} else {
				slice::from_raw_parts_mut(NonNull::dangling().as_ptr(), 0)
			}
		}
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

		unsafe {
			ptr::read(self.data.unwrap().as_ptr())
		}
	}

	/// Returns the first element of the vector.
	pub fn last(&mut self) -> T {
		if self.is_empty() {
			self.vector_panic(0);
		}

		unsafe {
			ptr::read(self.data.unwrap().as_ptr().offset((self.len - 1) as _))
		}
	}

	/// Inserts an element at position index within the vector, shifting all elements after it to
	/// the right.
	pub fn insert(&mut self, index: usize, element: T) -> Result<(), Errno> {
		if self.capacity < self.len + 1 {
			self.increase_capacity(self.capacity + 1)?;
		}
		debug_assert!(self.capacity >= self.len + 1);

		let ptr = self.data.unwrap().as_ptr();
		unsafe {
			ptr::copy(ptr.offset(index as _), ptr.offset((index + 1) as _), self.len - index);
			ptr::write(ptr.offset(index as _), element);
		}
		self.len += 1;
		Ok(())
	}

	/// Removes and returns the element at position index within the vector, shifting all elements
	/// after it to the left.
	pub fn remove(&mut self, index: usize) -> T {
		if self.is_empty() {
			self.vector_panic(0);
		}

		let ptr = self.data.unwrap().as_ptr();
		let v = unsafe {
			ptr::read(ptr.offset(index as _))
		};
		unsafe {
			ptr::copy(ptr.offset((index + 1) as _), ptr.offset(index as _), self.len - index - 1);
		}
		self.len -= 1;

		v
	}

	/// Moves all the elements of `other` into `Self`, leaving `other` empty.
	pub fn append(&mut self, other: &mut Vec::<T>) -> Result<(), Errno> {
		if self.capacity < self.len + other.len {
			self.increase_capacity(self.capacity + other.len)?;
		}

		let self_ptr = self.data.unwrap().as_ptr();
		let other_ptr = other.data.unwrap().as_ptr();
		unsafe {
			ptr::copy(other_ptr, self_ptr.offset(self.len as _), other.len);
		}

		self.len += other.len;
		other.clear();

		Ok(())
	}

	// TODO reserve
	// TODO resize

	/// Appends an element to the back of a collection.
	pub fn push(&mut self, value: T) -> Result<(), Errno> {
		if self.capacity < self.len + 1 {
			self.increase_capacity(self.capacity + 1)?;
		}
		debug_assert!(self.capacity >= self.len + 1);

		unsafe {
			ptr::write(self.data.unwrap().as_ptr().offset(self.len as _), value);
		}
		self.len += 1;
		Ok(())
	}

	/// Removes the last element from a vector and returns it, or None if it is empty.
	pub fn pop(&mut self) -> Option<T> {
		if !self.is_empty() {
			self.len -= 1;
			unsafe {
				Some(ptr::read(self.data.unwrap().as_ptr().offset(self.len as _)))
			}
		} else {
			None
		}
	}

	/// Creates an immutable iterator.
	pub fn iter(&self) -> VecIterator<'_, T> {
		VecIterator::new(self)
	}

	/// Clears the vector, removing all values.
	pub fn clear(&mut self) {
		for e in self.into_iter() {
			drop(e);
		}

		self.len = 0;
		self.capacity = 0;

		if self.data.is_some() {
			malloc::free(self.data.unwrap().as_ptr() as _);
			self.data = None;
		}
	}
}

impl<T: PartialEq> PartialEq for Vec::<T> {
	fn eq(&self, other: &Vec::<T>) -> bool {
		if self.len() != other.len() {
			return false;
		}

		for i in 0..self.len() {
			if self[i] != other[i] {
				return false;
			}
		}

		true
	}
}

impl<T> FailableClone for Vec::<T> where T: FailableClone {
	/// Clones the vector and its content.
	fn failable_clone(&self) -> Result<Self, Errno> {
		let mut v = Self {
			len: self.len,
			capacity: self.capacity,
			data: NonNull::new(malloc::alloc(self.capacity)? as *mut T),
		};
		for i in 0..self.len() {
			v[i] = self[i].failable_clone()?;
		}
		Ok(v)
	}
}

impl<T> Index<usize> for Vec<T> {
	type Output = T;

	#[inline]
	fn index(&self, index: usize) -> &Self::Output {
		if index >= self.len() {
			self.vector_panic(index);
		}

		unsafe { // Safe because in range of the container
			&*self.data.unwrap().as_ptr().offset(index as _)
		}
	}
}

impl<T> IndexMut<usize> for Vec<T> {
	#[inline]
	fn index_mut(&mut self, index: usize) -> &mut Self::Output {
		if index >= self.len() {
			self.vector_panic(index);
		}

		unsafe { // Safe because in range of the container
			&mut *self.data.unwrap().as_ptr().offset(index as _)
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

		let mut l = 0;
		let mut r = self.len();

		while l < r {
			let i = (l + r) / 2;
			let ord = f(&self[i]);
			match ord {
				Ordering::Less => {
					l = i;
				},
				Ordering::Greater => {
					r = i;
				},
				_ => {
					break;
				},
			}
		}

		let i = (l + r) / 2;
		if f(&self[i]) == Ordering::Equal {
			Ok(i)
		} else {
			Err(i)
		}
	}
}

/// An iterator for the Vec structure.
pub struct VecIterator<'a, T> {
	/// The vector to iterate into.
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

#[cfg(test)]
mod test {
	use super::*;

	#[test_case]
	fn vec_insert_remove0() {
		let mut v = Vec::<usize>::new();
		debug_assert_eq!(v.len(), 0);

		for i in 0..100 {
			v.insert(i, i).unwrap();
			debug_assert_eq!(v.len(), i + 1);
			debug_assert_eq!(v[i], i);
		}

		for i in (0..100).rev() {
			debug_assert_eq!(v.remove(i), i);
			debug_assert_eq!(v.len(), i);
		}
	}

	// TODO More tests for insert/remove

	// TODO append

	// TODO reserve
	// TODO resize

	#[test_case]
	fn vec_push() {
		let mut v = Vec::<usize>::new();
		debug_assert_eq!(v.len(), 0);

		for i in 0..100 {
			v.push(i).unwrap();
			debug_assert_eq!(v.len(), i + 1);
			debug_assert_eq!(v[i], i);
		}
	}

	#[test_case]
	fn vec_push_clear() {
		let mut v = Vec::<usize>::new();
		debug_assert_eq!(v.len(), 0);

		for i in 0..100 {
			v.push(i).unwrap();
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
			v.push(i).unwrap();
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
			assert_eq!(v, 0);
		} else {
			assert!(false);
		}
	}

	#[test_case]
	fn vec_binary_search1() {
		let mut v = Vec::<usize>::new();
		v.push(0).unwrap();

		if let Ok(v) = v.binary_search(&0) {
			assert_eq!(v, 0);
		} else {
			assert!(false);
		}
	}

	#[test_case]
	fn vec_binary_search2() {
		let mut v = Vec::<usize>::new();
		v.push(1).unwrap();

		if let Err(v) = v.binary_search(&0) {
			assert_eq!(v, 0);
		} else {
			assert!(false);
		}
	}

	#[test_case]
	fn vec_binary_search3() {
		let mut v = Vec::<usize>::new();
		v.push(1).unwrap();
		v.push(2).unwrap();
		v.push(3).unwrap();

		if let Ok(v) = v.binary_search(&2) {
			assert_eq!(v, 1);
		} else {
			assert!(false);
		}
	}

	#[test_case]
	fn vec_binary_search4() {
		let mut v = Vec::<usize>::new();
		v.push(0).unwrap();
		v.push(2).unwrap();
		v.push(4).unwrap();
		v.push(6).unwrap();
		v.push(8).unwrap();

		if let Ok(v) = v.binary_search(&6) {
			assert_eq!(v, 3);
		} else {
			assert!(false);
		}
	}
}
