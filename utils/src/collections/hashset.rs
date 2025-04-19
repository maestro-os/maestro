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

//! A [`HashSet`] works just like a [`HashMap`], except there is only a key and no value.

use super::{
	hashmap,
	hashmap::{
		hash,
		hash::FxHasher,
		raw,
		raw::{CTRL_DELETED, CTRL_EMPTY},
		Entry, HashMap,
	},
};
use crate::{errno::CollectResult, TryClone};
use core::{
	alloc::AllocError,
	borrow::Borrow,
	fmt,
	hash::{Hash, Hasher},
	mem,
};
use utils::errno::AllocResult;

/// The implementation of the hash set.
pub struct HashSet<K: Eq + Hash, H: Default + Hasher = FxHasher>(HashMap<K, (), H>);

impl<K: Eq + Hash> Default for HashSet<K> {
	fn default() -> Self {
		Self::new()
	}
}

impl<K: Eq + Hash, H: Default + Hasher> HashSet<K, H> {
	/// Creates a new empty instance.
	pub const fn new() -> Self {
		Self(HashMap::new())
	}

	/// Creates a new instance with the given capacity in number of elements.
	pub fn with_capacity(capacity: usize) -> AllocResult<Self> {
		Ok(Self(HashMap::with_capacity(capacity)?))
	}

	/// Returns the number of elements in the hash set.
	#[inline]
	pub fn len(&self) -> usize {
		self.0.len()
	}

	/// Tells whether the hash set is empty.
	#[inline]
	pub fn is_empty(&self) -> bool {
		self.0.is_empty()
	}

	/// Returns the number of elements the set can hold without reallocating.
	#[inline]
	pub fn capacity(&self) -> usize {
		self.0.capacity()
	}

	/// Returns an immutable reference to the value matching `value`.
	///
	/// If the key isn't present, the function return `None`.
	pub fn get<Q: ?Sized + Hash + Eq>(&self, value: &Q) -> Option<&K>
	where
		K: Borrow<Q>,
	{
		let hash = hash::<_, H>(value);
		let (slot_off, occupied) = self.0.inner.find_slot(value, hash, false)?;
		if occupied {
			let slot = self.0.inner.get_slot(slot_off);
			Some(unsafe { slot.key.assume_init_ref() })
		} else {
			None
		}
	}

	/// Tells whether the hash set contains the given `value`.
	#[inline]
	pub fn contains<Q: ?Sized + Hash + Eq>(&self, value: &Q) -> bool
	where
		K: Borrow<Q>,
	{
		self.get(value).is_some()
	}

	/// Creates an iterator of immutable references over all elements.
	#[inline]
	pub fn iter(&self) -> Iter<K, H> {
		Iter {
			inner: self.0.iter(),
		}
	}

	/// Tries to reserve memory for at least `additional` more elements. The function might reserve
	/// more memory than necessary to avoid frequent re-allocations.
	///
	/// If the hash set already has enough capacity, the function does nothing.
	pub fn reserve(&mut self, additional: usize) -> AllocResult<()> {
		self.0.reserve(additional)
	}

	/// Inserts a new element into the hash set.
	///
	/// If the value was already present, the function returns the previous value.
	pub fn insert(&mut self, value: K) -> AllocResult<Option<K>> {
		match self.0.entry(value) {
			Entry::Occupied(e) => {
				let old = mem::replace(unsafe { e.inner.key.assume_init_mut() }, e.key);
				Ok(Some(old))
			}
			Entry::Vacant(e) => {
				e.insert(())?;
				Ok(None)
			}
		}
	}

	/// Removes an element from the hash set.
	///
	/// If the value was present, the function returns the previous.
	pub fn remove<Q: ?Sized + Hash + Eq>(&mut self, value: &Q) -> Option<K>
	where
		K: Borrow<Q>,
	{
		let hash = hash::<_, H>(&value);
		let (slot_off, occupied) = self.0.inner.find_slot(value, hash, false)?;
		if occupied {
			self.0.len -= 1;
			let (group, index) = raw::get_slot_position::<K, ()>(slot_off);
			// Update control byte
			let ctrl = self.0.inner.get_ctrl(group);
			let h2 = raw::group_match_unused(ctrl, false)
				.map(|_| CTRL_EMPTY)
				.unwrap_or(CTRL_DELETED);
			self.0.inner.set_ctrl(group, index, h2);
			// Return previous value
			let slot = self.0.inner.get_slot_mut(slot_off);
			unsafe { Some(slot.key.assume_init_read()) }
		} else {
			None
		}
	}

	/// Drops all elements from the hash set.
	pub fn clear(&mut self) {
		self.0.clear()
	}
}

impl<K: Eq + Hash + fmt::Debug, H: Default + Hasher> fmt::Debug for HashSet<K, H> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		fmt::Debug::fmt(&self.0, f)
	}
}

impl<K: Eq + Hash + TryClone<Error = E>, H: Default + Hasher, E: From<AllocError>> TryClone
	for HashSet<K, H>
{
	type Error = E;

	fn try_clone(&self) -> Result<Self, Self::Error> {
		let hm = self
			.0
			.iter()
			.map(|(e, v)| Ok((e.try_clone()?, v.try_clone()?)))
			.collect::<Result<CollectResult<_>, Self::Error>>()?
			.0?;
		Ok(Self(hm))
	}
}

/// Iterator of immutable references over a [`HashSet`].
///
/// This iterator does not guarantee any order since the [`HashSet`] itself does not store values
/// in a specific order.
pub struct Iter<'m, K: Hash + Eq, H: Default + Hasher> {
	inner: hashmap::Iter<'m, K, (), H>,
}

impl<'m, K: Hash + Eq, H: Default + Hasher> Iterator for Iter<'m, K, H> {
	type Item = &'m K;

	#[inline]
	fn next(&mut self) -> Option<Self::Item> {
		self.inner.next().map(|(k, _)| k)
	}

	#[inline]
	fn size_hint(&self) -> (usize, Option<usize>) {
		self.inner.size_hint()
	}

	#[inline]
	fn count(self) -> usize {
		self.inner.count()
	}
}
