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
	vec,
};
use core::{
	borrow::Borrow,
	fmt,
	hash::{Hash, Hasher},
	intrinsics::{likely, unlikely},
	iter::{FusedIterator, TrustedLen},
	marker::PhantomData,
	mem,
	mem::{size_of, size_of_val, MaybeUninit},
	ops::{BitAnd, Index, IndexMut},
	ptr,
	simd::{cmp::SimdPartialEq, u8x16},
};

/// Indicates a vacant entry in the map. This is a sentinel value for the lookup operation.
const CTRL_EMPTY: u8 = 0x80;
/// Indicates a deleted entry in the map.
const CTRL_DELETED: u8 = 0xfe;
/// The size of a group of entries.
const GROUP_SIZE: usize = 16;

/// Macro to get a mutable reference to a slot from the given `group` and `index`.
///
/// **Note**: This macro is a workaround to avoid borrow-checker issues.
macro_rules! get_slot {
	($data:expr, $off:expr) => {{
		unsafe { &*(&($data)[$off] as *const _ as *const Slot<K, V>) }
	}};
	($data:expr, $off:expr, mut) => {{
		unsafe { &mut *(&mut ($data)[$off] as *mut _ as *mut Slot<K, V>) }
	}};
}

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

/// Initializes a new data buffer with the given capacity.
fn init_data<K, V>(capacity: usize) -> AllocResult<Vec<u8>> {
	let capacity = capacity.next_multiple_of(GROUP_SIZE);
	let new_ctrl_off = (capacity * size_of::<Slot<K, V>>()).next_multiple_of(16);
	let new_size = new_ctrl_off + capacity;
	let mut data = vec![0u8; new_size]?;
	data[new_ctrl_off..].fill(CTRL_EMPTY);
	Ok(data)
}

#[inline]
fn capacity_impl<K, V>(data: &[u8]) -> usize {
	// `+ 1` for the control byte
	data.len() / (size_of::<Slot<K, V>>() + 1)
}

/// Returns the control block for the given `group`.
#[inline]
fn get_ctrl<K, V>(data: &[u8], group: usize) -> u8x16 {
	let ctrl_start =
		(capacity_impl::<K, V>(data) * size_of::<Slot<K, V>>()).next_multiple_of(GROUP_SIZE);
	let off = ctrl_start + group * GROUP_SIZE;
	u8x16::from_slice(&data[off..(off + GROUP_SIZE)])
}

/// Sets the control bytes for a slot.
#[inline]
fn set_ctrl<K, V>(data: &mut [u8], group: usize, index: usize, h2: u8) {
	let ctrl_start =
		(capacity_impl::<K, V>(data) * size_of::<Slot<K, V>>()).next_multiple_of(GROUP_SIZE);
	let off = ctrl_start + group * GROUP_SIZE + index;
	data[off] = h2;
}

/// Returns the hash for the given key.
fn hash<K: ?Sized + Hash, H: Default + Hasher>(key: &K) -> u64 {
	let mut hasher = H::default();
	key.hash(&mut hasher);
	hasher.finish()
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

/// Returns the offset to a slot for the given `group` and in-group-index `index`.
#[inline]
fn get_slot_offset<K, V>(group: usize, index: usize) -> usize {
	(group * GROUP_SIZE + index) * size_of::<Slot<K, V>>()
}

/// Returns the group and in-group-index for the slot at the given offset.
#[inline]
fn get_slot_position<K, V>(off: usize) -> (usize, usize) {
	let off = off / size_of::<Slot<K, V>>();
	(off / GROUP_SIZE, off % GROUP_SIZE)
}

/// Returns the first empty element of the given `group`.
#[inline]
fn group_match_empty(group: u8x16) -> Option<usize> {
	let mask = u8x16::splat(CTRL_EMPTY);
	let matching = group.simd_eq(mask);
	matching.first_set()
}

/// Returns the slot corresponding the given key and its hash.
///
/// Return tuple:
/// - The offset of the slot in the data buffer
/// - Whether the slot is occupied
fn find_slot<K, V, Q: ?Sized>(data: &[u8], key: &Q, hash: u64) -> Option<(usize, bool)>
where
	K: Borrow<Q>,
	Q: Eq,
{
	let groups_count = capacity_impl::<K, V>(data) / GROUP_SIZE;
	if groups_count == 0 {
		return None;
	}
	let start_group = (h1(hash) % groups_count as u64) as usize;
	let mut group = start_group;
	let find_mask = u8x16::splat(h2(hash));
	loop {
		// Find key in group
		let ctrl = get_ctrl::<K, V>(data, group);
		let matching = ctrl.simd_eq(find_mask);
		let iter = (0usize..GROUP_SIZE).filter(move |i| matching.test(*i));
		for i in iter {
			let slot_off = get_slot_offset::<K, V>(group, i);
			let slot = get_slot!(data, slot_off);
			let slot_key = unsafe { slot.key.assume_init_ref() };
			if likely(slot_key.borrow() == key) {
				return Some((slot_off, true));
			}
		}
		// Check for an empty slot
		if let Some(i) = group_match_empty(ctrl) {
			#[cold]
			return Some((get_slot_offset::<K, V>(group, i), false));
		}
		group = (group + 1) % groups_count;
		// If coming back to the first group
		if unlikely(group == start_group) {
			return None;
		}
	}
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
	/// The key to insert.
	key: K,
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
		let data = init_data::<K, V>(capacity)?;
		Ok(Self {
			data,
			len: 0,

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
		capacity_impl::<K, V>(&self.data)
	}

	/// Returns the entry for the given key.
	pub fn entry(&mut self, key: K) -> Entry<'_, K, V> {
		let hash = hash::<_, H>(&key);
		match find_slot::<K, V, _>(&self.data, &key, hash) {
			Some((slot_off, true)) => Entry::Occupied(OccupiedEntry {
				inner: get_slot!(self.data, slot_off, mut),
			}),
			Some((slot_off, false)) => Entry::Vacant(VacantEntry {
				key,
				inner: Some(get_slot!(self.data, slot_off, mut)),
			}),
			None => Entry::Vacant(VacantEntry {
				key,
				inner: None,
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
		let (slot_off, occupied) = find_slot::<K, V, Q>(&self.data, key, hash)?;
		let slot = get_slot!(self.data, slot_off);
		if occupied {
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
		let (slot_off, occupied) = find_slot::<K, V, Q>(&self.data, key, hash)?;
		let slot = get_slot!(self.data, slot_off, mut);
		if occupied {
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

			group: 0,
			cursor: 0,

			count: 0,
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
		let mut data = init_data::<K, V>(new_capacity)?;
		// Rehash
		for (k, v) in self.iter() {
			// Get slot for key
			let hash = hash::<_, H>(k);
			// Should not fail since the correct amount of slots has been allocated
			let (slot_off, occupied) = find_slot::<K, V, _>(&data, k, hash).unwrap();
			assert!(!occupied);
			// Update control block
			let (group, index) = get_slot_position::<K, V>(slot_off);
			set_ctrl::<K, V>(&mut data, group, index, h2(hash));
			let slot = get_slot!(data, slot_off, mut);
			// Insert key/value
			unsafe {
				slot.key.write(ptr::read(k));
				slot.value.write(ptr::read(v));
			}
		}
		// Replace, freeing the previous buffer without dropping thanks to `MaybeUninit`
		self.data = data;
		Ok(())
	}

	/// Inserts a new element into the hash map.
	///
	/// If the key was already present, the function returns the previous value.
	pub fn insert(&mut self, key: K, value: V) -> AllocResult<Option<V>> {
		let hash = hash::<_, H>(&key);
		match find_slot::<K, V, _>(&self.data, &key, hash) {
			// The entry already exists
			Some((slot_off, true)) => {
				let slot = get_slot!(self.data, slot_off, mut);
				// No need to replace the key because `key == old.key` and the transitivity
				// property holds, so future comparisons will be consistent
				Ok(Some(mem::replace(
					unsafe { slot.value.assume_init_mut() },
					value,
				)))
			}
			// The entry does not exist but a slot was found
			Some((slot_off, false)) => {
				self.len += 1;
				// Update control block
				let (group, index) = get_slot_position::<K, V>(slot_off);
				set_ctrl::<K, V>(&mut self.data, group, index, h2(hash));
				// Insert key/value
				let slot = get_slot!(self.data, slot_off, mut);
				slot.key.write(key);
				slot.value.write(value);
				Ok(None)
			}
			// The entry does not exist and no slot was found
			None => {
				// Allocate space, then retry
				self.reserve(1)?;
				// The insertion cannot fail because the container is guaranteed to have space for
				// the new object
				self.insert(key, value).unwrap();
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
		let (slot_off, occupied) = find_slot::<K, V, _>(&self.data, key, hash)?;
		if occupied {
			self.len -= 1;
			let (group, index) = get_slot_position::<K, V>(slot_off);
			// Update control byte
			let ctrl = get_ctrl::<K, V>(&self.data, group);
			let new = group_match_empty(ctrl)
				.map(|_| CTRL_EMPTY)
				.unwrap_or(CTRL_DELETED);
			set_ctrl::<K, V>(&mut self.data, group, index, new);
			// Return previous value
			let slot = get_slot!(self.data, slot_off, mut);
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
		let mask = u8x16::splat(0x80);
		for group in 0..groups_count {
			// Check whether there are elements in the group
			let ctrl = get_ctrl::<K, V>(&self.data, group);
			let matching = ctrl.bitand(mask).simd_ne(mask);
			let iter = (0..GROUP_SIZE).filter(move |i| matching.test(*i));
			// Iterate on slots in group
			for i in iter {
				let off = get_slot_offset::<K, V>(group, i);
				let slot = get_slot!(self.data, off, mut);
				let (key, value) =
					unsafe { (slot.key.assume_init_ref(), slot.value.assume_init_mut()) };
				let keep = f(key, value);
				if !keep {
					// TODO use CTRL_EMPTY if relevant
					set_ctrl::<K, V>(&mut self.data, group, i, CTRL_DELETED);
					self.len -= 1;
				}
			}
		}
	}

	/// Drops all elements in the hash map.
	pub fn clear(&mut self) {
		// TODO drop all keys and values
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

	/// The current group to iterate on.
	group: usize,
	/// The cursor in the group.
	cursor: usize,

	/// The number of elements iterated on so far.
	count: usize,
}

impl<'m, K: Hash + Eq, V, H: Default + Hasher> Iterator for Iter<'m, K, V, H> {
	type Item = (&'m K, &'m V);

	fn next(&mut self) -> Option<Self::Item> {
		let capacity = self.hm.capacity();
		// If no element remain, stop
		if self.group * GROUP_SIZE + self.cursor >= capacity {
			return None;
		}
		// Find next group with an element in it
		// TODO do not run each time
		let cursor = loop {
			let ctrl = get_ctrl::<K, V>(&self.hm.data, self.group);
			let mask = u8x16::splat(0x80);
			let matching = ctrl.bitand(mask).simd_ne(mask);
			let cursor = (self.cursor..GROUP_SIZE).find(move |i| matching.test(*i));
			if let Some(cursor) = cursor {
				break cursor;
			}
			self.group += 1;
			self.cursor = 0;
			// If no group remain
			if self.group >= capacity / GROUP_SIZE {
				return None;
			}
		};
		let off = get_slot_offset::<K, V>(self.group, cursor);
		let slot = get_slot!(self.hm.data, off);
		let (key, value) = unsafe { (slot.key.assume_init_ref(), slot.value.assume_init_ref()) };
		self.cursor = cursor + 1;
		self.count += 1;
		Some((key, value))
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		let remaining = self.hm.len - self.count;
		(remaining, Some(remaining))
	}

	fn count(self) -> usize {
		self.hm.len - self.count
	}
}

// TODO implement DoubleEndedIterator

impl<'m, K: Hash + Eq, V, H: Default + Hasher> ExactSizeIterator for Iter<'m, K, V, H> {
	fn len(&self) -> usize {
		self.size_hint().0
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

	#[test_case]
	fn hashmap1() {
		let mut hm = HashMap::<u32, u32>::new();

		for i in 0..100 {
			assert_eq!(hm.len(), i);

			hm.insert(i as _, i as _).unwrap();
			assert_eq!(hm.len(), i + 1);

			assert_eq!(*hm.get(&(i as _)).unwrap(), i as _);
			assert_eq!(hm[i as _], i as _);
		}

		for i in (0..100).rev() {
			assert_eq!(hm.len(), i + 1);
			assert_eq!(hm.remove(&(i as _)).unwrap(), i as _);
			assert_eq!(hm.len(), i);
		}
	}

	#[test_case]
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
