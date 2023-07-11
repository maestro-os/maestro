//! This module implements the Vec container.

use crate::errno::Errno;
use crate::memory::malloc;
use crate::util::TryClone;
use core::cmp::max;
use core::cmp::min;
use core::fmt;
use core::hash::Hash;
use core::hash::Hasher;
use core::ops::Deref;
use core::ops::DerefMut;
use core::ops::Index;
use core::ops::IndexMut;
use core::ops::Range;
use core::ops::RangeFrom;
use core::ops::RangeTo;
use core::ptr;
use core::ptr::drop_in_place;

/// Macro allowing to create a vector with the given set of values.
#[macro_export]
macro_rules! vec {
	// Creating an empty vec
	() => {
		$crate::util::container::vec::Vec::new()
	};

	// Creating a vec filled with `n` times `elem`
	($elem:expr; $n:expr) => (
		$crate::util::container::vec::Vec::from_elem($elem, $n)
	);

	// Creating a vec from the given slice
	($($x:expr), + $(,) ?) => {{
		let slice = [$($x),+];

		(|| {
			let mut v = $crate::util::container::vec::Vec::with_capacity(slice.len())?;
			for i in slice {
				v.push(i)?;
			}

			Ok(v)
		})()
	}};
}

/// A vector container is a dynamically-resizable array of elements.
///
/// When resizing a vector, the elements can be moved, thus the callee should
/// not rely on pointers to elements inside a vector.
///
/// The implementation of vectors for the kernel cannot follow the
/// implementation of Rust's standard Vec because it must not panic when a
/// memory allocation fails.
pub struct Vec<T> {
	/// The number of elements present in the vector
	len: usize,
	/// The vector's data
	data: Option<malloc::Alloc<T>>,
}

impl<T> Default for Vec<T> {
	fn default() -> Self {
		Self {
			len: 0,
			data: None,
		}
	}
}

impl<T> Vec<T> {
	/// Creates a new empty vector.
	pub const fn new() -> Self {
		Self {
			len: 0,
			data: None,
		}
	}

	/// Reallocates the vector's data with the vector's capacity.
	///
	/// `capacity` is the new capacity in number of elements.
	fn realloc(&mut self, capacity: usize) -> Result<(), Errno> {
		if capacity == 0 {
			self.data = None;
			return Ok(());
		}

		if let Some(data) = &mut self.data {
			debug_assert!(data.len() >= self.len);

			// Safe because the memory is rewritten when the object is placed into the
			// vector
			unsafe {
				data.realloc_zero(capacity)?;
			}
		} else {
			// Safe because the memory is rewritten when the object is placed into the
			// vector
			let data_ptr = unsafe { malloc::Alloc::new_zero(capacity)? };

			self.data = Some(data_ptr);
		};

		Ok(())
	}

	/// Increases the capacity of so that at least `min` more elements can fit.
	fn increase_capacity(&mut self, min: usize) -> Result<(), Errno> {
		if self.len + min <= self.capacity() {
			return Ok(());
		}

		let curr_capacity = self.capacity();
		// multiply capacity by 1.25
		let capacity = max(curr_capacity + (curr_capacity / 4), self.len + min);
		self.realloc(capacity)
	}

	/// Creates a new emoty vector with the given capacity.
	pub fn with_capacity(capacity: usize) -> Result<Self, Errno> {
		let mut vec = Self::new();
		vec.realloc(capacity)?;

		Ok(vec)
	}

	/// Returns the number of elements inside of the vector.
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
		self.data.as_ref().map(|d| d.len()).unwrap_or(0)
	}

	/// Returns a slice containing the data.
	pub fn as_slice(&self) -> &[T] {
		if let Some(p) = &self.data {
			&p.as_slice()[..self.len]
		} else {
			&[]
		}
	}

	/// Returns a mutable slice containing the data.
	pub fn as_mut_slice(&mut self) -> &mut [T] {
		if let Some(p) = &mut self.data {
			&mut p.as_slice_mut()[..self.len]
		} else {
			&mut []
		}
	}

	/// Triggers a panic after an invalid access to the vector.
	#[cold]
	fn vector_panic(&self, index: usize) -> ! {
		panic!(
			"index out of bounds: the len is {} but the index is {}",
			self.len, index
		);
	}

	/// Inserts an element at position index within the vector, shifting all
	/// elements after it to the right.
	///
	/// # Panics
	///
	/// Panics if `index > len`.
	pub fn insert(&mut self, index: usize, element: T) -> Result<(), Errno> {
		if index > self.len() {
			self.vector_panic(index);
		}

		self.increase_capacity(1)?;
		debug_assert!(self.capacity() > self.len);

		let data = self.data.as_mut().unwrap();
		unsafe {
			// Shift
			let ptr = data.as_ptr_mut();
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

		let data = self.data.as_mut().unwrap();
		let v = unsafe {
			let v = ptr::read(&data[index]);

			// Shift
			let ptr = data.as_ptr_mut();
			ptr::copy(ptr.add(index + 1), ptr.add(index), self.len - index - 1);

			v
		};

		self.len -= 1;
		v
	}

	/// Moves all the elements of `other` into `Self`, leaving `other` empty.
	pub fn append(&mut self, other: &mut Vec<T>) -> Result<(), Errno> {
		if other.is_empty() {
			return Ok(());
		}

		self.increase_capacity(other.len())?;

		unsafe {
			let self_ptr = self.data.as_mut().unwrap().as_ptr_mut();
			ptr::copy_nonoverlapping(other.as_ptr(), self_ptr.add(self.len), other.len());
		}

		self.len += other.len();

		// Clearing other without dropping its elements
		other.len = 0;
		other.data = None;

		Ok(())
	}

	/// Appends an element to the back of a collection.
	pub fn push(&mut self, value: T) -> Result<(), Errno> {
		self.increase_capacity(1)?;
		debug_assert!(self.capacity() > self.len);

		unsafe {
			ptr::write(&mut self.data.as_mut().unwrap()[self.len], value);
		}

		self.len += 1;
		Ok(())
	}

	/// Removes the last element from a vector and returns it, or `None` if it is
	/// empty.
	pub fn pop(&mut self) -> Option<T> {
		if !self.is_empty() {
			self.len -= 1;
			unsafe { Some(ptr::read(&self.data.as_ref().unwrap()[self.len])) }
		} else {
			None
		}
	}

	/// Creates an immutable iterator.
	pub fn iter(&self) -> VecIterator<'_, T> {
		VecIterator::new(self)
	}

	/// Retains only the elements for which the given closure returns `true`.
	///
	/// The function visit each elements exactly once, in order.
	pub fn retain<F: FnMut(&mut T) -> bool>(&mut self, mut f: F) {
		let len = self.len();
		let Some(data) = self.data.as_mut() else {
			return;
		};

		// The function looks for sequences of delete-keep groups, then shifts elements
		//
		// For example, for the following array:
		// [Keep, Delete, Delete, Keep, Keep, Delete]
		//
		// The sequence starts at element `1` and ends at element `4` (included)

		let mut processed = 0;
		let mut deleted_count = 0;
		let mut kept_count = 0;

		let mut new_len = 0;

		while processed < len {
			let cur = unsafe { &mut *data.as_ptr_mut().add(processed) };
			let keep = f(cur);
			processed += 1;

			if !keep {
				unsafe {
					ptr::drop_in_place(cur);
				}

				// If reaching the end of a delete-keep sequence, shift elements
				if kept_count > 0 {
					unsafe {
						let src = data.as_ptr().add(processed - kept_count - 1);
						let dst = data
							.as_ptr_mut()
							.add(processed - kept_count - deleted_count - 1);

						ptr::copy(src, dst, kept_count);
					}

					kept_count = 0;
				}

				deleted_count += 1;
			} else {
				if deleted_count > 0 {
					kept_count += 1;
				}

				new_len += 1;
			}
		}

		// If a sequence remains after the end, shift it
		if deleted_count > 0 && kept_count > 0 {
			unsafe {
				let src = data.as_ptr().add(processed - kept_count);
				let dst = data
					.as_ptr_mut()
					.add(processed - kept_count - deleted_count);

				ptr::copy(src, dst, kept_count);
			}
		}

		self.len = new_len;
	}

	/// Truncates the vector to the given new len `len`.
	///
	/// If `len` is greater than the current length, the function has no effect.
	pub fn truncate(&mut self, len: usize) {
		if len < self.len() {
			for e in &mut self.as_mut_slice()[len..] {
				unsafe {
					drop_in_place(e);
				}
			}

			self.len = len;
		}

		if len == 0 {
			self.data = None;
		}
	}

	/// Clears the vector, removing all values.
	pub fn clear(&mut self) {
		for e in self.as_mut_slice() {
			unsafe {
				drop_in_place(e);
			}
		}

		self.len = 0;
		self.data = None;
	}
}

impl<T: Default> Vec<T> {
	/// Resizes the vector to the given length `new_len`.
	///
	/// If new elements have to be created, the default value is used.
	pub fn resize(&mut self, new_len: usize) -> Result<(), Errno> {
		if new_len < self.len() {
			self.truncate(new_len);
		} else {
			self.increase_capacity(new_len - self.len)?;
			self.len = new_len;
		}

		Ok(())
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
		if self.len() != other.len() {
			return false;
		}

		self.iter().zip(other.iter()).all(|(e0, e1)| e0 == e1)
	}
}

impl<T: Clone> Vec<T> {
	/// Creates a new vector with `n` times `elem`.
	pub fn from_elem(elem: T, n: usize) -> Result<Self, Errno> {
		let mut v = Self::with_capacity(n)?;
		v.len = n;

		for i in 0..n {
			unsafe {
				// Safe because in range
				ptr::write(&mut v[i], elem.clone());
			}
		}

		Ok(v)
	}

	/// Creates a new vector from the given slice.
	pub fn from_slice(slice: &[T]) -> Result<Self, Errno> {
		let mut v = Vec::with_capacity(slice.len())?;
		v.len = slice.len();

		for (i, elem) in slice.iter().enumerate() {
			unsafe {
				// Safe because in range
				ptr::write(&mut v[i], elem.clone());
			}
		}

		Ok(v)
	}

	/// Extends the vector by cloning the elements from the given slice `slice`.
	pub fn extend_from_slice(&mut self, slice: &[T]) -> Result<(), Errno> {
		if slice.is_empty() {
			return Ok(());
		}

		self.increase_capacity(slice.len())?;
		for e in slice {
			self.push(e.clone())?;
		}

		Ok(())
	}
}

impl<T: TryClone> TryClone for Vec<T> {
	fn try_clone(&self) -> Result<Self, Errno> {
		let mut v = Self::with_capacity(self.len)?;

		for i in 0..self.len {
			let res: Result<_, Errno> = self[i].try_clone().map_err(Into::into);
			v.push(res?)?;
		}
		Ok(v)
	}
}

impl<T: TryClone> Vec<T> {
	/// Clones the vector, keeping the given range.
	pub fn clone_range(&self, range: Range<usize>) -> Result<Self, Errno> {
		let end = min(range.end, self.len);
		let start = min(range.start, range.end);
		let len = end - start;

		let mut v = Self::with_capacity(len)?;

		for i in 0..len {
			let res: Result<_, Errno> = self[start + i].try_clone().map_err(Into::into);
			v.push(res?)?;
		}
		Ok(v)
	}

	/// Clones the vector, keeping the given range.
	pub fn clone_range_from(&self, range: RangeFrom<usize>) -> Result<Self, Errno> {
		let len = self.len - min(self.len, range.start);
		let mut v = Self::with_capacity(len)?;

		for i in 0..len {
			let res: Result<_, Errno> = self[range.start + i].try_clone().map_err(Into::into);
			v.push(res?)?;
		}
		Ok(v)
	}

	/// Clones the vector, keeping the given range.
	pub fn clone_range_to(&self, range: RangeTo<usize>) -> Result<Self, Errno> {
		let len = min(self.len, range.end);
		let mut v = Self::with_capacity(len)?;

		for i in 0..len {
			let res: Result<_, Errno> = self[i].try_clone().map_err(Into::into);
			v.push(res?)?;
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

		&self.data.as_ref().unwrap()[index]
	}
}

impl<T> IndexMut<usize> for Vec<T> {
	#[inline]
	fn index_mut(&mut self, index: usize) -> &mut Self::Output {
		if index >= self.len() {
			self.vector_panic(index);
		}

		&mut self.data.as_mut().unwrap()[index]
	}
}

impl<T> Index<Range<usize>> for Vec<T> {
	type Output = [T];

	#[inline]
	fn index(&self, range: Range<usize>) -> &Self::Output {
		&self.as_slice()[range]
	}
}

impl<T> IndexMut<Range<usize>> for Vec<T> {
	#[inline]
	fn index_mut(&mut self, range: Range<usize>) -> &mut Self::Output {
		&mut self.as_mut_slice()[range]
	}
}

impl<T> Index<RangeFrom<usize>> for Vec<T> {
	type Output = [T];

	#[inline]
	fn index(&self, range: RangeFrom<usize>) -> &Self::Output {
		&self.as_slice()[range]
	}
}

impl<T> IndexMut<RangeFrom<usize>> for Vec<T> {
	#[inline]
	fn index_mut(&mut self, range: RangeFrom<usize>) -> &mut Self::Output {
		&mut self.as_mut_slice()[range]
	}
}

impl<T> Index<RangeTo<usize>> for Vec<T> {
	type Output = [T];

	#[inline]
	fn index(&self, range: RangeTo<usize>) -> &Self::Output {
		&self.as_slice()[range]
	}
}

impl<T> IndexMut<RangeTo<usize>> for Vec<T> {
	#[inline]
	fn index_mut(&mut self, range: RangeTo<usize>) -> &mut Self::Output {
		&mut self.as_mut_slice()[range]
	}
}

/// A consuming iterator for the Vec structure.
pub struct IntoIter<T> {
	/// The vector to iterator into.
	vec: Vec<T>,
}

impl<T> Iterator for IntoIter<T> {
	type Item = T;

	fn next(&mut self) -> Option<Self::Item> {
		self.vec.pop()
	}
}

impl<T> IntoIterator for Vec<T> {
	type IntoIter = IntoIter<T>;
	type Item = T;

	fn into_iter(self) -> Self::IntoIter {
		IntoIter {
			vec: self,
		}
	}
}

/// An iterator for the Vec structure.
pub struct VecIterator<'a, T> {
	/// The vector to iterate into.
	vec: &'a Vec<T>,

	/// The current index of the iterator starting from the beginning.
	index_front: usize,
	/// The current index of the iterator starting from the end.
	index_back: usize,
}

impl<'a, T> VecIterator<'a, T> {
	/// Creates a vector iterator for the given reference.
	fn new(vec: &'a Vec<T>) -> Self {
		VecIterator {
			vec,

			index_front: 0,
			index_back: 0,
		}
	}
}

impl<'a, T> Iterator for VecIterator<'a, T> {
	type Item = &'a T;

	fn next(&mut self) -> Option<Self::Item> {
		// If both ends of the iterator are meeting, stop
		if self.index_front + self.index_back >= self.vec.len() {
			return None;
		}

		if self.index_front < self.vec.len() {
			let e = &self.vec[self.index_front];
			self.index_front += 1;

			Some(e)
		} else {
			None
		}
	}

	fn count(self) -> usize {
		self.vec.len()
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		let remaining = self.vec.len() - self.index_front - self.index_back;
		(remaining, Some(remaining))
	}
}

impl<'a, T> DoubleEndedIterator for VecIterator<'a, T> {
	fn next_back(&mut self) -> Option<Self::Item> {
		// If both ends of the iterator are meeting, stop
		if self.index_front + self.index_back >= self.vec.len() {
			return None;
		}

		if self.index_back < self.vec.len() {
			let e = &self.vec[self.vec.len() - self.index_back - 1];
			self.index_back += 1;

			Some(e)
		} else {
			None
		}
	}
}

impl<'a, T> IntoIterator for &'a Vec<T> {
	type IntoIter = VecIterator<'a, T>;
	type Item = &'a T;

	fn into_iter(self) -> Self::IntoIter {
		VecIterator::new(self)
	}
}

impl<T: Hash> Hash for Vec<T> {
	fn hash<H: Hasher>(&self, state: &mut H) {
		for i in 0..self.len() {
			self[i].hash(state);
		}
	}
}

impl<T: fmt::Debug> fmt::Debug for Vec<T> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "[")?;

		for (i, e) in self.iter().enumerate() {
			if i + 1 < self.len() {
				write!(f, "{:?}, ", e)?;
			} else {
				write!(f, "{:?}", e)?;
			}
		}

		write!(f, "]")
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
			debug_assert_eq!(v[0], i);
			v.pop();
			debug_assert_eq!(v.len(), 0);
		}
	}

	#[test_case]
	fn vec_retain0() {
		let mut v = Vec::<usize>::new();

		v.retain(|_| true);
		assert!(v.is_empty());

		v.retain(|_| false);
		assert!(v.is_empty());
	}

	#[test_case]
	fn vec_retain1() {
		let v: Result<Vec<usize>, Errno> = vec![0usize, 1, 2, 3, 4];
		let mut v = v.unwrap();
		v.retain(|_| true);
		assert_eq!(v.as_slice(), &[0, 1, 2, 3, 4]);

		let v: Result<Vec<usize>, Errno> = vec![0usize, 1, 2, 3, 4];
		let mut v = v.unwrap();
		v.retain(|_| false);
		assert_eq!(v.as_slice(), &[]);
	}

	#[test_case]
	fn vec_retain2() {
		let v: Result<Vec<usize>, Errno> = vec![0usize, 1, 2, 3, 4];
		let mut v = v.unwrap();
		v.retain(|i| *i % 2 == 0);
		assert_eq!(v.as_slice(), &[0, 2, 4]);

		let v: Result<Vec<usize>, Errno> = vec![0usize, 1, 2, 3, 4];
		let mut v = v.unwrap();
		v.retain(|i| *i % 2 == 1);
		assert_eq!(v.as_slice(), &[1, 3]);
	}

	#[test_case]
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

	#[test_case]
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

	#[test_case]
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

	#[test_case]
	fn vec_truncate3() {
		let mut v = Vec::<usize>::new();
		v.truncate(10000);
		assert_eq!(v.len(), 0);
	}

	// TODO Test resize

	// TODO Test range functions
}
