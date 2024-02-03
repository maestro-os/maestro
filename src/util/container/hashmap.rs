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

use super::vec::Vec;
use crate::{
	errno::{AllocResult, CollectResult},
	util::{AllocError, TryClone},
};
use core::{
	borrow::Borrow,
	fmt,
	hash::{Hash, Hasher},
	intrinsics::{likely, size_of, unlikely},
	iter::{FusedIterator, TrustedLen},
	marker::PhantomData,
	mem,
	mem::{size_of_val, MaybeUninit},
	ops::{Index, IndexMut},
	simd::{cmp::SimdPartialEq, u8x16},
};

/// Indicates a vacant entry in the map. This is a sentinel value for the lookup operation.
const CTRL_EMPTY: u8 = 0x80;
/// Indicates a deleted entry in the map.
const CTRL_DELETED: u8 = 0xfe;
/// The size of a group of entries.
const GROUP_SIZE: usize = 16;

/// Bitwise XOR hasher.
#[derive(Default)]
struct XORHasher {
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

/// Returns the slot part of the hash.
#[inline]
fn h1(hash: u64) -> u64 {
	hash >> 7
}

/// Returns the control part of the hash.
#[inline]
fn h2(hash: u64) -> u8 {
	(hash & 0x7f) as _
}

/// Returns an iterator of elements matching the given `h2` in `group`.
#[inline]
fn group_match(group: u8x16, h2: u8) -> impl Iterator<Item = usize> {
	let mask = u8x16::splat(h2);
	let matching = group.simd_eq(mask);
	(0usize..16).filter(move |i| matching.test(*i))
}

/// Returns the first empty element of the given `group`.
#[inline]
fn group_match_empty(group: u8x16) -> Option<usize> {
	let mask = u8x16::splat(CTRL_EMPTY);
	let matching = group.simd_eq(mask);
	matching.first_set()
}

/// Internal representation of an entry.
struct Slot<K, V> {
	/// The key stored in the slot.
	key: MaybeUninit<K>,
	/// The value stored in the slot.
	value: MaybeUninit<V>,
}

/// TODO doc
pub struct OccupiedEntry<'h, K, V> {
	inner: &'h mut Slot<K, V>,
}

/// TODO doc
pub struct VacantEntry<'h, K, V> {
	/// The inner slot.
	///
	/// If `None`, the hash map requires resizing for the insertion.
	inner: Option<&'h mut Slot<K, V>>,
}

/// An entry in a hash map.
pub enum Entry<'h, K: Eq + Hash, V> {
	Occupied(OccupiedEntry<'h, K, V>),
	Vacant(VacantEntry<'h, K, V>),
}

/// The implementation of the hash map.
///
/// Underneath, it is an implementation of the [SwissTable](https://abseil.io/about/design/swisstables).
pub struct HashMap<K: Eq + Hash, V, H: Default + Hasher = XORHasher> {
	/// The map's data.
	///
	/// This vector is split in two parts:
	/// - Slots table: actual stored data
	/// - Control block: allowing for fast lookup into the table
	data: Vec<u8>,
	/// The number of elements in the map.
	len: usize,

	_key: PhantomData<K>,
	_val: PhantomData<V>,
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
			data: Vec::new(),
			len: 0,

			_key: PhantomData,
			_val: PhantomData,
			_hasher: PhantomData,
		}
	}

	/// Creates a new instance with the given capacity in number of elements.
	pub fn with_capacity(capacity: usize) -> AllocResult<Self> {
		let len = capacity * (size_of::<Slot<K, V>>() + 1);
		Ok(Self {
			data: Vec::with_capacity(len)?,
			len,

			_key: PhantomData,
			_val: PhantomData,
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
		// `+ 1` for the control byte
		self.data.len() / (size_of::<Slot<K, V>>() + 1)
	}

	/// Returns the slot for the given `group` and index `i` and the group.
	#[inline]
	fn get_slot(&mut self, group: usize, i: usize) -> &mut Slot<K, V> {
		let off = (group * GROUP_SIZE + i) * size_of::<Slot<K, V>>();
		unsafe { &mut *(&mut self.data[off] as *mut _ as *mut Slot<K, V>) }
	}

	/// Returns the control block for the given `group`.
	#[inline]
	fn get_ctrl(&self, group: usize) -> u8x16 {
		let ctrl_start = self.capacity() * size_of::<Slot<K, V>>();
		// TODO add padding for alignment?
		let off = ctrl_start + group * GROUP_SIZE;
		u8x16::from_slice(&self.data[off..(off + GROUP_SIZE)])
	}

	/// Returns the entry for the given key.
	pub fn entry<Q: ?Sized>(&mut self, key: &Q) -> Entry<'_, K, V>
	where
		K: Borrow<Q>,
		Q: Hash + Eq,
	{
		// Hash key
		let mut hasher = H::default();
		key.hash(&mut hasher);
		let hash = hasher.finish();
		// Search groups
		let groups_count = self.capacity() / GROUP_SIZE;
		let start_group = (h1(hash) % groups_count as u64) as usize;
		let mut group = start_group;
		loop {
			// Find key in group
			let ctrl = self.get_ctrl(group);
			for i in group_match(ctrl, h2(hash)) {
				let slot = self.get_slot(group, i);
				let slot_key = unsafe { slot.key.assume_init_ref() }.borrow();
				if likely(slot_key == key) {
					return Entry::Occupied(OccupiedEntry {
						inner: slot,
					});
				}
			}
			// Check for an empty slot
			if let Some(i) = group_match_empty(ctrl) {
				#[cold]
				{
					let slot = self.get_slot(group, i);
					return Entry::Vacant(VacantEntry {
						inner: Some(slot),
					});
				}
			}
			group = (group + 1) % groups_count;
			if unlikely(group == start_group) {
				break;
			}
		}
		Entry::Vacant(VacantEntry {
			inner: None,
		})
	}

	/// Returns an immutable reference to the value with the given `key`.
	///
	/// If the key isn't present, the function return `None`.
	pub fn get<Q: ?Sized>(&self, _key: &Q) -> Option<&V>
	where
		K: Borrow<Q>,
		Q: Hash + Eq,
	{
		// TODO
		todo!()
	}

	/// Returns a mutable reference to the value with the given `key`.
	///
	/// If the key isn't present, the function return `None`.
	pub fn get_mut<Q: ?Sized>(&mut self, key: &Q) -> Option<&mut V>
	where
		K: Borrow<Q>,
		Q: Hash + Eq,
	{
		let Entry::Occupied(entry) = self.entry(key) else {
			return None;
		};
		Some(unsafe { entry.inner.value.assume_init_mut() })
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
		}
	}

	/// Tries to reserve memory for at least `additional` more elements.
	///
	/// If the hash map already has enough capacity, the function does nothing.
	pub fn reserve(&mut self, _additional: usize) -> AllocResult<()> {
		// TODO
		todo!()
	}

	/// Inserts a new element into the hash map.
	///
	/// If the key was already present, the function returns the previous value.
	pub fn insert(&mut self, key: K, value: V) -> AllocResult<Option<V>> {
		let entry = self.entry(&key);
		match entry {
			// The entry already exists
			Entry::Occupied(old) => {
				// No need to replace the key because `key == old.key` and the transitivity
				// property holds, so future comparisons will be consistent
				Ok(Some(mem::replace(
					unsafe { old.inner.value.assume_init_mut() },
					value,
				)))
			}
			// The entry does not exist but a slot was found
			Entry::Vacant(VacantEntry {
				// TODO update ctrl block
				inner: Some(Slot {
					key: k,
					value: v,
				}),
			}) => {
				k.write(key);
				v.write(value);
				Ok(None)
			}
			// The entry does not exist and no slot was found
			Entry::Vacant(VacantEntry {
				inner: None,
			}) => {
				// TODO
				Ok(None)
			}
		}
	}

	/// Removes an element from the hash map.
	///
	/// If the key was present, the function returns the previous value.
	pub fn remove<Q: ?Sized>(&mut self, _key: &Q) -> Option<V>
	where
		K: Borrow<Q>,
		Q: Hash + Eq,
	{
		// TODO
		todo!()
	}

	/// Retains only the elements for which the given predicate returns `true`.
	pub fn retain<F: FnMut(&K, &mut V) -> bool>(&mut self, mut _f: F) {
		// TODO
		todo!()
	}

	/// Drops all elements in the hash map.
	pub fn clear(&mut self) {
		self.data.clear();
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

impl<
		K: Eq + Hash + TryClone<Error = E>,
		V: TryClone<Error = E>,
		H: Default + Hasher,
		E: From<AllocError>,
	> TryClone for HashMap<K, V, H>
{
	type Error = E;

	fn try_clone(&self) -> Result<Self, Self::Error> {
		Ok(Self {
			data: self.data.try_clone()?,
			len: self.len,

			_key: PhantomData,
			_val: PhantomData,
			_hasher: PhantomData,
		})
	}
}

/// Iterator for the [`HashMap`] structure.
///
/// This iterator doesn't guarantee any order since the HashMap itself doesn't store value in a
/// specific order.
pub struct Iter<'m, K: Hash + Eq, V, H: Default + Hasher> {
	/// The hash map to iterate into.
	hm: &'m HashMap<K, V, H>,
	// TODO
}

impl<'m, K: Hash + Eq, V, H: Default + Hasher> Iterator for Iter<'m, K, V, H> {
	type Item = (&'m K, &'m V);

	fn next(&mut self) -> Option<Self::Item> {
		// TODO
		todo!()
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		// TODO
		todo!()
	}

	fn count(self) -> usize {
		// TODO
		todo!()
	}
}

// TODO implement DoubleEndedIterator

impl<'m, K: Hash + Eq, V, H: Default + Hasher> ExactSizeIterator for Iter<'m, K, V, H> {
	fn len(&self) -> usize {
		self.hm.len()
	}
}

impl<'m, K: Hash + Eq, V, H: Default + Hasher> FusedIterator for Iter<'m, K, V, H> {}

unsafe impl<'m, K: Hash + Eq, V, H: Default + Hasher> TrustedLen for Iter<'m, K, V, H> {}

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

#[cfg(test)]
mod test {
	use super::*;

	#[test_case]
	fn hash_map0() {
		let mut hash_map = HashMap::<u32, u32>::new();

		assert_eq!(hash_map.len(), 0);

		hash_map.insert(0, 0).unwrap();

		assert_eq!(hash_map.len(), 1);
		assert_eq!(*hash_map.get(&0).unwrap(), 0);
		assert_eq!(hash_map[0], 0);

		assert_eq!(hash_map.remove(&0).unwrap(), 0);

		assert_eq!(hash_map.len(), 0);
	}

	#[test_case]
	fn hash_map1() {
		let mut hash_map = HashMap::<u32, u32>::new();

		for i in 0..100 {
			assert_eq!(hash_map.len(), i);

			hash_map.insert(i as _, 0).unwrap();

			assert_eq!(hash_map.len(), i + 1);
			assert_eq!(*hash_map.get(&(i as _)).unwrap(), 0);
			assert_eq!(hash_map[i as _], 0);
		}

		for i in (0..100).rev() {
			assert_eq!(hash_map.len(), i + 1);
			assert_eq!(hash_map.remove(&(i as _)).unwrap(), 0);
			assert_eq!(hash_map.len(), i);
		}
	}
}
