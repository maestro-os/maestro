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

//! A dynamically-resizable array of elements.

use crate::{
	errno::{AllocResult, CollectResult},
	TryClone,
};
use alloc::alloc::Global;
use core::{
	alloc::{AllocError, Allocator, Layout},
	cmp::max,
	fmt,
	hash::{Hash, Hasher},
	iter::{FusedIterator, TrustedLen},
	mem::ManuallyDrop,
	ops::{Deref, DerefMut, Index, IndexMut, Range, RangeFrom, RangeTo},
	ptr,
	ptr::NonNull,
	slice,
};

/// Inner vector, used to manage allocations.
struct RawVec<T> {
	/// A pointer to the first element of the vector.
	data: NonNull<T>,
	/// The capacity of the vector.
	capacity: usize,
}

impl<T> RawVec<T> {
	/// Creates a new empty instance.
	const fn new() -> Self {
		Self {
			data: NonNull::dangling(),
			capacity: 0,
		}
	}

	fn as_slice(&self) -> &[T] {
		if self.capacity > 0 {
			unsafe { NonNull::slice_from_raw_parts(self.data, self.capacity).as_ref() }
		} else {
			&[]
		}
	}

	fn as_mut_slice(&mut self) -> &mut [T] {
		if self.capacity > 0 {
			unsafe { NonNull::slice_from_raw_parts(self.data, self.capacity).as_mut() }
		} else {
			&mut []
		}
	}

	/// Reallocates the vector's data with the given new `capacity`.
	fn realloc(&mut self, capacity: usize) -> AllocResult<()> {
		if capacity == 0 {
			// Free remaining memory
			self.free();
			return Ok(());
		}
		let new_layout = Layout::array::<T>(capacity).map_err(|_| AllocError)?;
		let new = if self.capacity > 0 {
			let old_layout = Layout::array::<T>(self.capacity).unwrap();
			// SAFETY: memory is rewritten when the object is placed into the vector
			unsafe { Global.grow(self.data.cast(), old_layout, new_layout)? }.cast()
		} else {
			Global.allocate(new_layout)?.cast()
		};
		self.data = new;
		self.capacity = capacity;
		Ok(())
	}

	/// Frees the underlying memory.
	fn free(&mut self) {
		if self.capacity == 0 {
			return;
		}
		let layout = Layout::array::<T>(self.capacity).unwrap();
		// SAFETY: the underlying data is no longer used
		unsafe {
			Global.deallocate(self.data.cast(), layout);
		}
		self.capacity = 0;
	}
}

impl<T> Drop for RawVec<T> {
	fn drop(&mut self) {
		self.free();
	}
}

/// Creates a [`Vec`] with the given size or set of values.
#[macro_export]
macro_rules! vec {
	// Create an empty vec
	() => {
		$crate::collections::vec::Vec::new()
	};
	// Create a vec filled with `n` times `elem`
	($elem:expr; $n:expr) => {{
		let mut v = $crate::collections::vec::Vec::new();
		v.resize($n, $elem)?;
		$crate::errno::AllocResult::Ok(v)
	}};
	// Create a vec from the given array
	($($x:expr), + $(,) ?) => {{
		let array = [$($x),+];
		(|| {
			let mut v = $crate::collections::vec::Vec::with_capacity(array.len())?;
			for i in array {
				v.push(i)?;
			}
			$crate::errno::AllocResult::Ok(v)
		})()
	}};
}

/// A vector collection is a dynamically-resizable array of elements.
///
/// When resizing a vector, the elements may be moved, thus the callee should
/// not rely on pointers to elements inside a vector.
///
/// The implementation of vectors for the kernel cannot follow the implementation of Rust's
/// standard `Vec` because it must provide a way to recover from memory allocation failures.
pub struct Vec<T> {
	/// The inner vector.
	inner: RawVec<T>,
	/// The number of elements present in the vector
	len: usize,
}

impl<T> Default for Vec<T> {
	fn default() -> Self {
		Self::new()
	}
}

impl<T> Vec<T> {
	/// Creates a new empty vector.
	pub const fn new() -> Self {
		Self {
			inner: RawVec::new(),
			len: 0,
		}
	}

	/// Reserves capacity for at least `additional` more element to be inserted.
	pub fn reserve(&mut self, additional: usize) -> AllocResult<()> {
		if self.len + additional <= self.capacity() {
			return Ok(());
		}
		let curr_capacity = self.capacity();
		// multiply capacity by 1.25
		let capacity = max(curr_capacity + (curr_capacity / 4), self.len + additional);
		self.inner.realloc(capacity)
	}

	/// Creates a new empty vector with the given capacity.
	pub fn with_capacity(capacity: usize) -> AllocResult<Self> {
		let mut vec = Self::new();
		vec.inner.realloc(capacity)?;
		Ok(vec)
	}

	/// Returns the number of elements inside the vector.
	#[inline(always)]
	pub fn len(&self) -> usize {
		self.len
	}

	/// Returns `true` if the vector contains no elements.
	#[inline(always)]
	pub fn is_empty(&self) -> bool {
		self.len == 0
	}

	/// Returns the number of elements that can be stored inside of the vector
	/// without needing to reallocate the memory.
	#[inline(always)]
	pub fn capacity(&self) -> usize {
		self.inner.capacity
	}

	/// Returns a slice containing the data.
	pub fn as_slice(&self) -> &[T] {
		&self.inner.as_slice()[..self.len]
	}

	/// Returns a mutable slice containing the data.
	pub fn as_mut_slice(&mut self) -> &mut [T] {
		&mut self.inner.as_mut_slice()[..self.len]
	}

	/// Triggers a panic after invalid access to the vector.
	#[cold]
	fn vector_panic(&self, index: usize) -> ! {
		panic!(
			"index out of bounds: the len is {len} but the index is {index}",
			len = self.len
		);
	}

	/// Inserts an element at position index within the vector, shifting all
	/// elements after it to the right.
	///
	/// # Panics
	///
	/// Panics if `index > len`.
	pub fn insert(&mut self, index: usize, element: T) -> AllocResult<()> {
		if index > self.len() {
			self.vector_panic(index);
		}
		self.reserve(1)?;
		let data = self.inner.as_mut_slice();
		unsafe {
			// Shift
			let ptr = data.as_mut_ptr();
			ptr::copy(ptr.add(index), ptr.add(index + 1), self.len - index);
			ptr::write(&mut data[index], element);
		}
		self.len += 1;
		Ok(())
	}

	/// Removes and returns the element at position index within the vector,
	/// shifting all elements after it to the left.
	///
	/// # Panics
	///
	/// Panics if `index >= len`.
	pub fn remove(&mut self, index: usize) -> T {
		if index >= self.len() {
			self.vector_panic(index);
		}
		let data = self.inner.as_mut_slice();
		let v = unsafe {
			let v = ptr::read(&data[index]);
			// Shift
			let ptr = data.as_mut_ptr();
			ptr::copy(ptr.add(index + 1), ptr.add(index), self.len - index - 1);
			v
		};
		self.len -= 1;
		v
	}

	/// Moves all the elements of `other` into `Self`, leaving `other` empty.
	pub fn append(&mut self, other: &mut Vec<T>) -> AllocResult<()> {
		if other.is_empty() {
			return Ok(());
		}
		self.reserve(other.len())?;
		unsafe {
			let self_ptr = self.inner.as_mut_slice().as_mut_ptr();
			ptr::copy_nonoverlapping(other.as_ptr(), self_ptr.add(self.len), other.len());
		}
		self.len += other.len();
		// Clear other without dropping its elements
		other.len = 0;
		other.inner.free();
		Ok(())
	}

	/// Appends an element to the back of a collection.
	pub fn push(&mut self, value: T) -> AllocResult<()> {
		self.reserve(1)?;
		unsafe {
			ptr::write(&mut self.inner.as_mut_slice()[self.len], value);
		}
		self.len += 1;
		Ok(())
	}

	/// Removes the last element from a vector and returns it, or `None` if it is
	/// empty.
	pub fn pop(&mut self) -> Option<T> {
		if !self.is_empty() {
			self.len -= 1;
			unsafe { Some(ptr::read(&self.inner.as_slice()[self.len])) }
		} else {
			None
		}
	}

	/// Retains only the elements for which the given closure returns `true`.
	///
	/// The function visit each element exactly once, in order.
	pub fn retain<F: FnMut(&mut T) -> bool>(&mut self, mut f: F) {
		let len = self.len();
		/*
		 * The function looks for sequences of delete-keep groups, then shifts elements
		 *
		 * For example, for the following array:
		 * [Keep, Delete, Delete, Keep, Keep, Delete]
		 *
		 * The sequence starts at element `1` and ends at element `4` (included)
		 */
		let mut deleted = 0;
		let mut kept = 0;
		let mut new_len = 0;
		for i in 0..=len {
			let slice = self.as_mut_slice();
			let keep = slice
				.get_mut(i)
				.map(|e| {
					let keep = f(e);
					if !keep {
						unsafe {
							ptr::drop_in_place(e);
						}
					}
					keep
				})
				.unwrap_or(false);
			// If reaching the end of a delete-keep sequence, shift elements
			if kept > 0 && deleted > 0 && !keep {
				/*
				 * SAFETY:
				 * - elements to be clobbered have already been dropped by `drop_in_place`
				 *   before
				 * - both `src` and `dst` are guaranteed to stay in bound as `i >= kept +
				 *   deleted` is always `true`
				 */
				unsafe {
					let ptr = slice.as_mut_ptr();
					let src = ptr.add(i - kept);
					let dst = ptr.add(i - kept - deleted);
					ptr::copy(src, dst, kept);
				}
				kept = 0;
			}
			if !keep {
				deleted += 1;
			} else {
				if deleted > 0 {
					kept += 1;
				}
				new_len += 1;
			}
		}
		self.len = new_len;
	}

	/// Truncates the vector to the given new len `len`.
	///
	/// If `len` is greater than or equal to the current length, the function has no effect.
	pub fn truncate(&mut self, len: usize) {
		if len == 0 {
			self.clear();
			return;
		}
		if len < self.len() {
			for e in &mut self.as_mut_slice()[len..] {
				unsafe {
					ptr::drop_in_place(e);
				}
			}
			self.len = len;
		}
	}

	/// Clears the vector, removing all values.
	pub fn clear(&mut self) {
		for e in self.as_mut_slice() {
			unsafe {
				ptr::drop_in_place(e);
			}
		}
		self.len = 0;
		self.inner.free();
	}
}

impl<T> FromIterator<T> for CollectResult<Vec<T>> {
	fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
		let mut iter = iter.into_iter();
		let min_size = iter.size_hint().0;
		let res = (|| {
			let mut vec = Vec::with_capacity(min_size)?;
			vec.len = min_size;
			// push elements in the range of minimum size
			for (dst, elem) in vec
				.inner
				.as_mut_slice()
				.iter_mut()
				.zip(iter.by_ref().take(min_size))
			{
				unsafe {
					ptr::write(dst, elem);
				}
			}
			// push remaining elements
			for elem in iter {
				vec.push(elem)?;
			}
			Ok(vec)
		})();
		Self(res)
	}
}

impl<'a, T: 'a + Clone> FromIterator<&'a T> for CollectResult<Vec<T>> {
	fn from_iter<I: IntoIterator<Item = &'a T>>(iter: I) -> Self {
		CollectResult::<Vec<T>>::from_iter(iter.into_iter().cloned())
	}
}

impl<T> AsRef<[T]> for Vec<T> {
	fn as_ref(&self) -> &[T] {
		self.as_slice()
	}
}

impl<T> AsMut<[T]> for Vec<T> {
	fn as_mut(&mut self) -> &mut [T] {
		self.as_mut_slice()
	}
}

impl<T> Deref for Vec<T> {
	type Target = [T];

	fn deref(&self) -> &Self::Target {
		self.as_slice()
	}
}

impl<T> DerefMut for Vec<T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		self.as_mut_slice()
	}
}

impl<T: Eq> Eq for Vec<T> {}

impl<T: PartialEq> PartialEq for Vec<T> {
	fn eq(&self, other: &Vec<T>) -> bool {
		PartialEq::eq(&**self, &**other)
	}
}

impl<T: Clone> Vec<T> {
	/// Resizes the vector to the given length `new_len` with the `value` used for all the new
	/// elements.
	///
	/// If the new length is lower than the current, the size of the vector is truncated.
	///
	/// If new elements have to be created, the default value is used.
	pub fn resize(&mut self, new_len: usize, value: T) -> AllocResult<()> {
		if new_len < self.len() {
			self.truncate(new_len);
		} else {
			self.reserve(new_len - self.len)?;
			let old_len = self.len;
			self.len = new_len;
			for e in &mut self.as_mut_slice()[old_len..new_len] {
				// Safe because in range
				unsafe {
					// This is necessary to avoid dropping
					ptr::write(e, value.clone());
				}
			}
		}
		Ok(())
	}

	/// Creates a new vector from the given slice.
	pub fn from_slice(slice: &[T]) -> AllocResult<Self> {
		let mut v = Vec::with_capacity(slice.len())?;
		v.len = slice.len();
		for (i, elem) in slice.iter().enumerate() {
			// Safe because in range
			unsafe {
				// This is necessary to avoid dropping
				ptr::write(&mut v[i], elem.clone());
			}
		}
		Ok(v)
	}

	/// Extends the vector by cloning the elements from the given slice `slice`.
	pub fn extend_from_slice(&mut self, slice: &[T]) -> AllocResult<()> {
		if slice.is_empty() {
			return Ok(());
		}
		self.reserve(slice.len())?;
		let begin = self.len;
		self.len += slice.len();
		for (i, elem) in slice.iter().enumerate() {
			// Safe because in range
			unsafe {
				// This is necessary to avoid dropping
				ptr::write(&mut self[begin + i], elem.clone());
			}
		}
		Ok(())
	}
}

impl<T: TryClone<Error = E>, E: From<AllocError>> TryClone for Vec<T> {
	type Error = E;

	fn try_clone(&self) -> Result<Self, Self::Error> {
		let mut v = Self::with_capacity(self.len)?;
		v.len = self.len;
		for i in 0..self.len {
			// Safe because in range
			unsafe {
				// This is necessary to avoid dropping
				ptr::write(&mut v[i], self[i].try_clone()?);
			}
		}
		Ok(v)
	}
}

impl<T> Index<usize> for Vec<T> {
	type Output = T;

	#[inline]
	fn index(&self, index: usize) -> &Self::Output {
		Index::index(&**self, index)
	}
}

impl<T> IndexMut<usize> for Vec<T> {
	#[inline]
	fn index_mut(&mut self, index: usize) -> &mut Self::Output {
		IndexMut::index_mut(&mut **self, index)
	}
}

impl<T> Index<Range<usize>> for Vec<T> {
	type Output = [T];

	#[inline]
	fn index(&self, range: Range<usize>) -> &Self::Output {
		Index::index(&**self, range)
	}
}

impl<T> IndexMut<Range<usize>> for Vec<T> {
	#[inline]
	fn index_mut(&mut self, range: Range<usize>) -> &mut Self::Output {
		IndexMut::index_mut(&mut **self, range)
	}
}

impl<T> Index<RangeFrom<usize>> for Vec<T> {
	type Output = [T];

	#[inline]
	fn index(&self, range: RangeFrom<usize>) -> &Self::Output {
		Index::index(&**self, range)
	}
}

impl<T> IndexMut<RangeFrom<usize>> for Vec<T> {
	#[inline]
	fn index_mut(&mut self, range: RangeFrom<usize>) -> &mut Self::Output {
		IndexMut::index_mut(&mut **self, range)
	}
}

impl<T> Index<RangeTo<usize>> for Vec<T> {
	type Output = [T];

	#[inline]
	fn index(&self, range: RangeTo<usize>) -> &Self::Output {
		Index::index(&**self, range)
	}
}

impl<T> IndexMut<RangeTo<usize>> for Vec<T> {
	#[inline]
	fn index_mut(&mut self, range: RangeTo<usize>) -> &mut Self::Output {
		IndexMut::index_mut(&mut **self, range)
	}
}

impl<T> IntoIterator for Vec<T> {
	type IntoIter = IntoIter<T>;
	type Item = T;

	fn into_iter(self) -> Self::IntoIter {
		let end = self.len();
		IntoIter {
			vec: ManuallyDrop::new(self),
			start: 0,
			end,
		}
	}
}

impl<'a, T> IntoIterator for &'a Vec<T> {
	type IntoIter = slice::Iter<'a, T>;
	type Item = &'a T;

	fn into_iter(self) -> Self::IntoIter {
		self.as_slice().iter()
	}
}

impl<'a, T> IntoIterator for &'a mut Vec<T> {
	type IntoIter = slice::IterMut<'a, T>;
	type Item = &'a mut T;

	fn into_iter(self) -> Self::IntoIter {
		self.as_mut_slice().iter_mut()
	}
}

impl<T: Hash> Hash for Vec<T> {
	fn hash<H: Hasher>(&self, state: &mut H) {
		for e in self {
			e.hash(state);
		}
	}
}

impl<T: fmt::Debug> fmt::Debug for Vec<T> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt::Debug::fmt(&**self, f)
	}
}

impl<T> Drop for Vec<T> {
	fn drop(&mut self) {
		self.clear();
	}
}

/// A consuming iterator over [`Vec`].
pub struct IntoIter<T> {
	/// The vector to iterate onto.
	vec: ManuallyDrop<Vec<T>>,
	/// The current start offset in the vector.
	start: usize,
	/// The current end offset in the vector.
	end: usize,
}

impl<T> Iterator for IntoIter<T> {
	type Item = T;

	fn next(&mut self) -> Option<Self::Item> {
		// Fuse invariant
		if self.start >= self.end {
			return None;
		}
		// Read one element and return it
		let e = unsafe { ptr::read(&self.vec[self.start]) };
		self.start += 1;
		Some(e)
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		let len = self.end - self.start;
		(len, Some(len))
	}

	fn count(self) -> usize {
		self.size_hint().0
	}
}

impl<T> DoubleEndedIterator for IntoIter<T> {
	fn next_back(&mut self) -> Option<Self::Item> {
		// Fuse invariant
		if self.start >= self.end {
			return None;
		}
		// Read one element and return it
		self.end -= 1;
		Some(unsafe { ptr::read(&self.vec[self.end]) })
	}
}

impl<T> ExactSizeIterator for IntoIter<T> {}

impl<T> FusedIterator for IntoIter<T> {}

unsafe impl<T> TrustedLen for IntoIter<T> {}

impl<T> Drop for IntoIter<T> {
	fn drop(&mut self) {
		// Drop remaining elements
		for e in &mut self.vec.as_mut_slice()[self.start..self.end] {
			unsafe {
				ptr::drop_in_place(e);
			}
		}
		self.vec.inner.free();
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
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

	#[test]
	fn vec_push() {
		let mut v = Vec::<usize>::new();
		debug_assert_eq!(v.len(), 0);

		for i in 0..100 {
			v.push(i).unwrap();
			debug_assert_eq!(v.len(), i + 1);
			debug_assert_eq!(v[i], i);
		}
	}

	#[test]
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

	#[test]
	fn vec_push_pop() {
		let mut v = Vec::<usize>::new();
		debug_assert_eq!(v.len(), 0);

		for i in 0..100 {
			v.push(i).unwrap();
			debug_assert_eq!(v.len(), 1);
			debug_assert_eq!(v[0], i);
			v.pop();
			debug_assert_eq!(v.len(), 0);
		}
	}

	#[test]
	fn vec_iter() {
		const COUNT: usize = 1000;
		let v = (0..COUNT).collect::<CollectResult<Vec<_>>>().0.unwrap();
		let iter = v.into_iter();
		assert_eq!(iter.size_hint().0, COUNT);
		for (i, j) in iter.zip(0..COUNT) {
			assert_eq!(i, j);
		}
		// Reverse order
		let v = (0..COUNT).collect::<CollectResult<Vec<_>>>().0.unwrap();
		let iter = v.into_iter();
		assert_eq!(iter.size_hint().0, COUNT);
		for (i, j) in iter.rev().zip((0..COUNT).rev()) {
			assert_eq!(i, j);
		}
	}

	#[test]
	fn vec_retain0() {
		let mut v = Vec::<usize>::new();

		v.retain(|_| true);
		assert!(v.is_empty());

		v.retain(|_| false);
		assert!(v.is_empty());
	}

	#[test]
	fn vec_retain1() {
		let mut v: Vec<usize> = vec![0usize, 1, 2, 3, 4].unwrap();
		v.retain(|_| true);
		assert_eq!(v.as_slice(), &[0, 1, 2, 3, 4]);

		let mut v: Vec<usize> = vec![0usize, 1, 2, 3, 4].unwrap();
		v.retain(|_| false);
		assert_eq!(v.as_slice(), &[]);
	}

	#[test]
	fn vec_retain2() {
		let mut v: Vec<usize> = vec![0usize, 1, 2, 3, 4].unwrap();
		v.retain(|i| *i % 2 == 0);
		assert_eq!(v.as_slice(), &[0, 2, 4]);

		let mut v: Vec<usize> = vec![0usize, 1, 2, 3, 4].unwrap();
		v.retain(|i| *i % 2 == 1);
		assert_eq!(v.as_slice(), &[1, 3]);
	}

	#[test]
	fn vec_truncate0() {
		let mut v = Vec::<usize>::new();
		v.push(0).unwrap();
		v.push(2).unwrap();
		v.push(4).unwrap();
		v.push(6).unwrap();
		v.push(8).unwrap();

		v.truncate(0);
		assert!(v.is_empty());
	}

	#[test]
	fn vec_truncate1() {
		let mut v = Vec::<usize>::new();
		v.push(0).unwrap();
		v.push(2).unwrap();
		v.push(4).unwrap();
		v.push(6).unwrap();
		v.push(8).unwrap();

		v.truncate(1);
		assert_eq!(v.len(), 1);
		assert_eq!(v[0], 0);
	}

	#[test]
	fn vec_truncate2() {
		let mut v = Vec::<usize>::new();
		v.push(0).unwrap();
		v.push(2).unwrap();
		v.push(4).unwrap();
		v.push(6).unwrap();
		v.push(8).unwrap();

		for i in (0..=5).rev() {
			v.truncate(i);
			assert_eq!(v.len(), i);
		}
	}

	#[test]
	fn vec_truncate3() {
		let mut v = Vec::<usize>::new();
		v.truncate(10000);
		assert_eq!(v.len(), 0);
	}

	// TODO Test resize
}
