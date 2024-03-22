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

//! A hashmap is a data structure that stores key/value pairs into buckets and
//! uses the hash of the key to quickly get the bucket storing the value.

mod raw;

use crate::{
	collections::hashmap::raw::{RawTable, CTRL_DELETED, CTRL_EMPTY, GROUP_SIZE},
	errno::{AllocResult, CollectResult},
	TryClone,
};
use core::{
	alloc::AllocError,
	borrow::Borrow,
	fmt,
	hash::{Hash, Hasher},
	iter::{FusedIterator, TrustedLen},
	marker::PhantomData,
	mem,
	mem::size_of_val,
	ops::{BitAnd, Index, IndexMut},
	ptr,
	simd::{cmp::SimdPartialEq, u8x16, Mask},
};
use raw::Slot;

/// Bitwise XOR hasher.
#[derive(Default)]
pub struct XORHasher {
	/// The currently stored value.
	value: u64,
	/// The offset byte at which the next XOR operation shall be performed.
	off: u8,
}

impl Hasher for XORHasher {
	fn finish(&self) -> u64 {
		self.value
	}

	fn write(&mut self, bytes: &[u8]) {
		for b in bytes {
			self.value ^= (*b as u64) << (self.off * 8);
			self.off = (self.off + 1) % size_of_val(&self.value) as u8;
		}
	}
}

/// Returns the hash for the given key.
pub fn hash<K: ?Sized + Hash, H: Default + Hasher>(key: &K) -> u64 {
	let mut hasher = H::default();
	key.hash(&mut hasher);
	hasher.finish()
}

/// Occupied entry in the hashmap.
pub struct OccupiedEntry<'h, K, V> {
	inner: &'h mut Slot<K, V>,
}

impl<'h, K: Eq + Hash, V> OccupiedEntry<'h, K, V> {
	/// Returns a mutable reference to the value.
	pub fn get_mut(&mut self) -> &mut V {
		unsafe { self.inner.value.assume_init_mut() }
	}

	/// Converts the [`OccupiedEntry`] into a mutable reference to the value in the entry with a
	/// lifetime bound to the map itself.
	pub fn into_mut(self) -> &'h mut V {
		unsafe { self.inner.value.assume_init_mut() }
	}

	/// Sets the value of the entry, and returns the entry's old value.
	pub fn insert(&mut self, value: V) -> V {
		mem::replace(unsafe { self.inner.value.assume_init_mut() }, value)
	}
}

/// Vacant entry in the hashmap.
pub struct VacantEntry<'h, K: Eq + Hash, V, H: Default + Hasher> {
	/// The hashmap containing the entry.
	hm: &'h mut HashMap<K, V, H>,
	/// The key to insert.
	key: K,
	/// The hash of the key.
	hash: u64,
	/// The offset of the inner slot.
	///
	/// If `None`, the hash map requires resizing for the insertion.
	slot_off: Option<usize>,
}

impl<'h, K: Eq + Hash, V, H: Default + Hasher> VacantEntry<'h, K, V, H> {
	/// Sets the value of the entry and returns a mutable reference to it.
	pub fn insert(self, value: V) -> AllocResult<&'h mut V> {
		let slot_off = match self.slot_off {
			Some(slot_off) => slot_off,
			None => {
				// Allocate space for the new object
				self.hm.reserve(1)?;
				// Cannot fail because the collection is guaranteed to have space for the new
				// object
				let (slot_off, occupied) =
					self.hm.inner.find_slot(&self.key, self.hash, true).unwrap();
				debug_assert!(!occupied);
				slot_off
			}
		};
		self.hm.len += 1;
		// Update control block
		let (group, index) = raw::get_slot_position::<K, V>(slot_off);
		self.hm.inner.set_ctrl(group, index, raw::h2(self.hash));
		// Insert key/value
		let slot = self.hm.inner.get_slot_mut(slot_off);
		slot.key.write(self.key);
		Ok(slot.value.write(value))
	}
}

/// An entry in a hash map.
pub enum Entry<'h, K: Eq + Hash, V, H: Default + Hasher> {
	Occupied(OccupiedEntry<'h, K, V>),
	Vacant(VacantEntry<'h, K, V, H>),
}

impl<'h, K: Eq + Hash, V, H: Default + Hasher> Entry<'h, K, V, H> {
	/// Ensures a value is in the entry by inserting the default if empty, and returns a mutable
	/// reference to the value in the entry.
	pub fn or_insert(self, default: V) -> AllocResult<&'h mut V> {
		match self {
			Entry::Occupied(e) => Ok(e.into_mut()),
			Entry::Vacant(e) => e.insert(default),
		}
	}
}

/// The implementation of the hash map.
///
/// Underneath, it is an implementation of the [SwissTable](https://abseil.io/about/design/swisstables).
pub struct HashMap<K: Eq + Hash, V, H: Default + Hasher = XORHasher> {
	/// The inner table.
	inner: RawTable<K, V>,
	/// The number of elements in the map.
	len: usize,
	_hasher: PhantomData<H>,
}

impl<K: Eq + Hash, V> Default for HashMap<K, V> {
	fn default() -> Self {
		Self::new()
	}
}

impl<K: Eq + Hash, V, const N: usize> TryFrom<[(K, V); N]> for HashMap<K, V> {
	type Error = AllocError;

	fn try_from(arr: [(K, V); N]) -> Result<Self, Self::Error> {
		arr.into_iter().collect::<CollectResult<_>>().0
	}
}

impl<K: Eq + Hash, V, H: Default + Hasher> HashMap<K, V, H> {
	/// Creates a new empty instance.
	pub const fn new() -> Self {
		Self {
			inner: RawTable::new(),
			len: 0,
			_hasher: PhantomData,
		}
	}

	/// Creates a new instance with the given capacity in number of elements.
	pub fn with_capacity(capacity: usize) -> AllocResult<Self> {
		Ok(Self {
			inner: RawTable::with_capacity(capacity)?,
			len: 0,
			_hasher: PhantomData,
		})
	}

	/// Returns the number of elements in the hash map.
	#[inline]
	pub fn len(&self) -> usize {
		self.len
	}

	/// Tells whether the hash map is empty.
	#[inline]
	pub fn is_empty(&self) -> bool {
		self.len == 0
	}

	/// Returns the number of elements the map can hold without reallocating.
	#[inline]
	pub fn capacity(&self) -> usize {
		self.inner.capacity()
	}

	/// Returns the entry for the given key.
	pub fn entry(&mut self, key: K) -> Entry<'_, K, V, H> {
		let hash = hash::<_, H>(&key);
		match self.inner.find_slot(&key, hash, true) {
			Some((slot_off, true)) => Entry::Occupied(OccupiedEntry {
				inner: self.inner.get_slot_mut(slot_off),
			}),
			Some((slot_off, false)) => Entry::Vacant(VacantEntry {
				hm: self,
				key,
				hash,
				slot_off: Some(slot_off),
			}),
			None => Entry::Vacant(VacantEntry {
				hm: self,
				key,
				hash,
				slot_off: None,
			}),
		}
	}

	/// Returns an immutable reference to the value with the given `key`.
	///
	/// If the key isn't present, the function return `None`.
	pub fn get<Q: ?Sized>(&self, key: &Q) -> Option<&V>
	where
		K: Borrow<Q>,
		Q: Hash + Eq,
	{
		let hash = hash::<_, H>(key);
		let (slot_off, occupied) = self.inner.find_slot(key, hash, false)?;
		if occupied {
			let slot = self.inner.get_slot(slot_off);
			Some(unsafe { slot.value.assume_init_ref() })
		} else {
			None
		}
	}

	/// Returns a mutable reference to the value with the given `key`.
	///
	/// If the key isn't present, the function return `None`.
	pub fn get_mut<Q: ?Sized>(&mut self, key: &Q) -> Option<&mut V>
	where
		K: Borrow<Q>,
		Q: Hash + Eq,
	{
		let hash = hash::<_, H>(key);
		let (slot_off, occupied) = self.inner.find_slot(key, hash, false)?;
		if occupied {
			let slot = self.inner.get_slot_mut(slot_off);
			Some(unsafe { slot.value.assume_init_mut() })
		} else {
			None
		}
	}

	/// Tells whether the hash map contains the given `key`.
	#[inline]
	pub fn contains_key<Q: ?Sized>(&self, k: &Q) -> bool
	where
		K: Borrow<Q>,
		Q: Hash + Eq,
	{
		self.get(k).is_some()
	}

	/// Creates an iterator of immutable references for the hash map.
	#[inline]
	pub fn iter(&self) -> Iter<K, V, H> {
		Iter {
			hm: self,
			inner: IterInner {
				group: 0,
				group_used: Mask::default(),
				cursor: 0,

				count: 0,
			},
		}
	}

	/// Tries to reserve memory for at least `additional` more elements. The function might reserve
	/// more memory than necessary to avoid frequent re-allocations.
	///
	/// If the hash map already has enough capacity, the function does nothing.
	pub fn reserve(&mut self, additional: usize) -> AllocResult<()> {
		// Compute new capacity
		let new_capacity = (self.len + additional).next_power_of_two();
		if self.capacity() >= new_capacity {
			return Ok(());
		}
		// Create new vector
		let mut new_table = RawTable::with_capacity(new_capacity)?;
		// Rehash
		for (k, v) in self.iter() {
			// Get slot for key
			let hash = hash::<_, H>(k);
			// Should not fail since the correct amount of slots has been allocated
			let (slot_off, occupied) = new_table.find_slot(k, hash, true).unwrap();
			debug_assert!(!occupied);
			// Update control block
			let (group, index) = raw::get_slot_position::<K, V>(slot_off);
			new_table.set_ctrl(group, index, raw::h2(hash));
			let slot = new_table.get_slot_mut(slot_off);
			// Insert key/value
			unsafe {
				slot.key.write(ptr::read(k));
				slot.value.write(ptr::read(v));
			}
		}
		// Replace, freeing the previous buffer without dropping elements thanks to `MaybeUninit`
		self.inner = new_table;
		Ok(())
	}

	/// Inserts a new element into the hash map.
	///
	/// If the key was already present, the function returns the previous value.
	pub fn insert(&mut self, key: K, value: V) -> AllocResult<Option<V>> {
		match self.entry(key) {
			Entry::Occupied(mut e) => Ok(Some(e.insert(value))),
			Entry::Vacant(e) => {
				e.insert(value)?;
				Ok(None)
			}
		}
	}

	/// Removes an element from the hash map.
	///
	/// If the key was present, the function returns the previous value.
	pub fn remove<Q: ?Sized>(&mut self, key: &Q) -> Option<V>
	where
		K: Borrow<Q>,
		Q: Hash + Eq,
	{
		let hash = hash::<_, H>(&key);
		let (slot_off, occupied) = self.inner.find_slot(key, hash, false)?;
		if occupied {
			self.len -= 1;
			let (group, index) = raw::get_slot_position::<K, V>(slot_off);
			// Update control byte
			let ctrl = self.inner.get_ctrl(group);
			let h2 = raw::group_match_unused(ctrl, false)
				.map(|_| CTRL_EMPTY)
				.unwrap_or(CTRL_DELETED);
			self.inner.set_ctrl(group, index, h2);
			// Return previous value
			let slot = self.inner.get_slot_mut(slot_off);
			unsafe {
				slot.key.assume_init_drop();
				Some(slot.value.assume_init_read())
			}
		} else {
			None
		}
	}

	// TODO merge implementation with mutable iterator?
	/// Retains only the elements for which the given predicate returns `true`.
	pub fn retain<F: FnMut(&K, &mut V) -> bool>(&mut self, mut f: F) {
		let groups_count = self.capacity() / GROUP_SIZE;
		for group in 0..groups_count {
			// Mask for values to be removed in the group
			let mut remove_mask: u16 = 0;
			let mut remove_count = 0;
			// Check whether there are elements in the group
			let ctrl = self.inner.get_ctrl(group);
			// The value to set in the group on remove
			let h2 = raw::group_match_unused(ctrl, false)
				.map(|_| CTRL_EMPTY)
				.unwrap_or(CTRL_DELETED);
			// Iterate on slots in group
			for i in raw::group_match_used(ctrl) {
				let slot_off = raw::get_slot_offset::<K, V>(group, i);
				let slot = self.inner.get_slot_mut(slot_off);
				let (key, value) =
					unsafe { (slot.key.assume_init_ref(), slot.value.assume_init_mut()) };
				let keep = f(key, value);
				if !keep {
					remove_mask |= 1 << i;
					remove_count += 1;
					unsafe {
						slot.key.assume_init_drop();
						slot.value.assume_init_drop();
					}
				}
			}
			// Update control block
			if remove_count > 0 {
				for i in 0..GROUP_SIZE {
					let set = remove_mask & (1 << i) != 0;
					if set {
						self.inner.set_ctrl(group, i, h2);
					}
				}
				self.len -= remove_count;
			}
		}
	}

	/// Drops all elements in the hash map.
	pub fn clear(&mut self) {
		// Drop everything
		self.retain(|_, _| false);
		self.len = 0;
	}
}

impl<K: Eq + Hash, V, H: Default + Hasher> Index<K> for HashMap<K, V, H> {
	type Output = V;

	#[inline]
	fn index(&self, k: K) -> &Self::Output {
		self.get(&k).expect("no entry found for key")
	}
}

impl<K: Eq + Hash, V, H: Default + Hasher> IndexMut<K> for HashMap<K, V, H> {
	#[inline]
	fn index_mut(&mut self, k: K) -> &mut Self::Output {
		self.get_mut(&k).expect("no entry found for key")
	}
}

impl<K: Eq + Hash, V, H: Default + Hasher> FromIterator<(K, V)>
	for CollectResult<HashMap<K, V, H>>
{
	fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
		let res = (|| {
			let iter = iter.into_iter();
			let capacity = iter.size_hint().0;
			let mut map = HashMap::with_capacity(capacity)?;
			for (key, value) in iter {
				map.insert(key, value)?;
			}
			Ok(map)
		})();
		Self(res)
	}
}

impl<K: Eq + Hash, V, H: Default + Hasher> IntoIterator for HashMap<K, V, H> {
	type IntoIter = IntoIter<K, V, H>;
	type Item = (K, V);

	fn into_iter(self) -> Self::IntoIter {
		IntoIter {
			hm: self,
			inner: IterInner {
				group: 0,
				group_used: Mask::default(),
				cursor: 0,

				count: 0,
			},
		}
	}
}

impl<K: Eq + Hash + fmt::Debug, V: fmt::Debug, H: Default + Hasher> fmt::Debug
	for HashMap<K, V, H>
{
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "[")?;
		for (i, (key, value)) in self.iter().enumerate() {
			write!(f, "{key:?}: {value:?}")?;
			if i + 1 < self.len() {
				write!(f, ", ")?;
			}
		}
		write!(f, "]")
	}
}

impl<
		K: Eq + Hash + TryClone<Error = E>,
		V: TryClone<Error = E>,
		H: Default + Hasher,
		E: From<AllocError>,
	> TryClone for HashMap<K, V, H>
{
	type Error = E;

	fn try_clone(&self) -> Result<Self, Self::Error> {
		Ok(self
			.iter()
			.map(|(e, v)| Ok((e.try_clone()?, v.try_clone()?)))
			.collect::<Result<CollectResult<Self>, Self::Error>>()?
			.0?)
	}
}

impl<K: Eq + Hash, V, H: Default + Hasher> Drop for HashMap<K, V, H> {
	fn drop(&mut self) {
		self.clear();
	}
}

/// Iterators logic.
struct IterInner {
	/// The current group to iterate on.
	group: usize,
	/// The current group's control block.
	group_used: Mask<i8, GROUP_SIZE>,
	/// The cursor in the group.
	cursor: usize,

	/// The number of elements iterated on so far.
	count: usize,
}

impl IterInner {
	/// Returns a tuple with the ID of the group and the cursor in that group, representing
	/// position of the next element to iterate on.
	///
	/// If no element is left, the function returns `None`.
	fn next_pos<K: Eq + Hash, V, H: Default + Hasher>(
		&mut self,
		hm: &HashMap<K, V, H>,
	) -> Option<(usize, usize)> {
		let capacity = hm.capacity();
		// If no element remain, stop
		if self.group * GROUP_SIZE + self.cursor >= capacity {
			return None;
		}
		// Find next group with an element in it
		let cursor = loop {
			// If at beginning of group, search for used elements
			if self.cursor == 0 {
				let ctrl = hm.inner.get_ctrl(self.group);
				let mask = u8x16::splat(0x80);
				self.group_used = ctrl.bitand(mask).simd_ne(mask);
			}
			if let Some(cursor) = self.group_used.first_set() {
				self.group_used.set(cursor, false);
				break cursor;
			}
			// No element has been found, go to next group
			self.group += 1;
			self.cursor = 0;
			// If no group remain
			if self.group >= capacity / GROUP_SIZE {
				return None;
			}
		};
		// Step cursor
		self.cursor = cursor + 1;
		self.count += 1;
		Some((self.group, cursor))
	}
}

/// Consuming iterator over the [`HashMap`] structure.
pub struct IntoIter<K: Eq + Hash, V, H: Default + Hasher> {
	/// The hash map.
	hm: HashMap<K, V, H>,
	/// Iterator logic.
	inner: IterInner,
}

impl<K: Hash + Eq, V, H: Default + Hasher> Iterator for IntoIter<K, V, H> {
	type Item = (K, V);

	fn next(&mut self) -> Option<Self::Item> {
		let (group, cursor) = self.inner.next_pos(&self.hm)?;
		let off = raw::get_slot_offset::<K, V>(group, cursor);
		let slot = self.hm.inner.get_slot(off);
		let (key, value) = unsafe { (slot.key.assume_init_read(), slot.value.assume_init_read()) };
		Some((key, value))
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		let remaining = self.hm.len - self.inner.count;
		(remaining, Some(remaining))
	}

	fn count(self) -> usize {
		self.size_hint().0
	}
}
impl<K: Hash + Eq, V, H: Default + Hasher> ExactSizeIterator for IntoIter<K, V, H> {}

impl<K: Hash + Eq, V, H: Default + Hasher> FusedIterator for IntoIter<K, V, H> {}

unsafe impl<K: Hash + Eq, V, H: Default + Hasher> TrustedLen for IntoIter<K, V, H> {}

impl<K: Hash + Eq, V, H: Default + Hasher> Drop for IntoIter<K, V, H> {
	fn drop(&mut self) {
		// Drop remaining elements
		for _ in self.by_ref() {}
		// Prevent double drop when dropping the hashmap
		mem::take(&mut self.hm.inner);
	}
}

/// Iterator of immutable references over the [`HashMap`] structure.
///
/// This iterator doesn't guarantee any order since the HashMap itself doesn't store value in a
/// specific order.
pub struct Iter<'m, K: Hash + Eq, V, H: Default + Hasher> {
	/// The hash map to iterate into.
	hm: &'m HashMap<K, V, H>,
	/// Iterator logic.
	inner: IterInner,
}

impl<'m, K: Hash + Eq, V, H: Default + Hasher> Iterator for Iter<'m, K, V, H> {
	type Item = (&'m K, &'m V);

	fn next(&mut self) -> Option<Self::Item> {
		let (group, cursor) = self.inner.next_pos(self.hm)?;
		let off = raw::get_slot_offset::<K, V>(group, cursor);
		let slot = self.hm.inner.get_slot(off);
		let (key, value) = unsafe { (slot.key.assume_init_ref(), slot.value.assume_init_ref()) };
		Some((key, value))
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		let remaining = self.hm.len - self.inner.count;
		(remaining, Some(remaining))
	}

	fn count(self) -> usize {
		self.size_hint().0
	}
}

impl<'m, K: Hash + Eq, V, H: Default + Hasher> ExactSizeIterator for Iter<'m, K, V, H> {}

impl<'m, K: Hash + Eq, V, H: Default + Hasher> FusedIterator for Iter<'m, K, V, H> {}

unsafe impl<'m, K: Hash + Eq, V, H: Default + Hasher> TrustedLen for Iter<'m, K, V, H> {}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn hashmap0() {
		let mut hm = HashMap::<u32, u32>::new();
		assert_eq!(hm.len(), 0);

		hm.insert(0, 0).unwrap();
		assert_eq!(hm.len(), 1);

		assert_eq!(*hm.get(&0).unwrap(), 0);
		assert_eq!(hm[0], 0);

		assert_eq!(hm.remove(&0).unwrap(), 0);
		assert_eq!(hm.len(), 0);
	}

	#[test]
	fn hashmap1() {
		let mut hm = HashMap::<u32, u32>::new();

		for i in 0..100 {
			assert_eq!(hm.len(), i);
			hm.insert(i as _, i as _).unwrap();
			assert_eq!(hm.len(), i + 1);
		}
		for i in 0..100 {
			assert_eq!(*hm.get(&(i as _)).unwrap(), i as _);
			assert_eq!(hm[i as _], i as _);
		}
		assert_eq!(hm.get(&100), None);
		for i in (0..100).rev() {
			assert_eq!(hm.len(), i + 1);
			assert_eq!(hm.remove(&(i as _)).unwrap(), i as _);
			assert_eq!(hm.len(), i);
		}
	}

	#[test]
	fn hashmap_retain() {
		let mut hm = (0..1000)
			.map(|i| (i, i))
			.collect::<CollectResult<HashMap<u32, u32>>>()
			.0
			.unwrap();
		assert_eq!(hm.len(), 1000);
		let mut next = 0;
		hm.retain(|i, j| {
			assert_eq!(*i, *j);
			assert_eq!(*i, next);
			next += 1;
			i % 2 == 0
		});
		assert_eq!(hm.len(), 500);
		hm.iter().for_each(|(i, _)| assert_eq!(i % 2, 0));
	}
}
