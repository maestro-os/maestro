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

//! Least Recently Used cache implementation.

use crate::{__alloc, __dealloc, collections::hashmap::HashSet, errno::AllocResult};
use core::{
	alloc::Layout,
	borrow::Borrow,
	hash::{Hash, Hasher},
	mem,
	mem::MaybeUninit,
	num::NonZeroUsize,
	ptr::NonNull,
};

/// Wrapper allowing to use the key inside an entry as the key for the [`HashSet`].
struct KeyHash<K, V>(NonNull<LruEntry<K, V>>);

impl<K, V> KeyHash<K, V> {
	/// Returns an immutable reference to the underlying entry.
	fn inner(&self) -> &LruEntry<K, V> {
		unsafe { self.0.as_ref() }
	}

	/// Returns a mutable reference to the underlying entry.
	#[allow(clippy::mut_from_ref)]
	unsafe fn inner_mut(&self) -> &mut LruEntry<K, V> {
		&mut *self.0.as_ptr()
	}
}

impl<K: Borrow<Q>, Q: ?Sized, V> Borrow<KeyWrapper<Q>> for KeyHash<K, V> {
	fn borrow(&self) -> &KeyWrapper<Q> {
		unsafe { KeyWrapper::from_ref(self.inner().key.assume_init_ref().borrow()) }
	}
}

impl<K: Eq, V> Eq for KeyHash<K, V> {}

impl<K: PartialEq, V> PartialEq for KeyHash<K, V> {
	fn eq(&self, other: &Self) -> bool {
		unsafe {
			self.inner()
				.key
				.assume_init_ref()
				.eq(other.inner().key.assume_init_ref())
		}
	}
}

impl<K: Hash, V> Hash for KeyHash<K, V> {
	fn hash<H: Hasher>(&self, state: &mut H) {
		unsafe { self.inner().key.assume_init_ref().hash(state) }
	}
}

/// Wrapper to allow blanket implementation of `Borrow` without conflicting with the stdlib's
/// implementation.
#[repr(transparent)]
struct KeyWrapper<K: ?Sized>(K);

impl<K: ?Sized> KeyWrapper<K> {
	fn from_ref(key: &K) -> &Self {
		// safety: KeyWrapper is transparent, so casting the ref like this is allowable
		unsafe { &*(key as *const K as *const KeyWrapper<K>) }
	}
}

impl<K: ?Sized + Hash> Hash for KeyWrapper<K> {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.0.hash(state)
	}
}

impl<K: ?Sized + PartialEq> PartialEq for KeyWrapper<K> {
	fn eq(&self, other: &Self) -> bool {
		self.0.eq(&other.0)
	}
}

impl<K: ?Sized + Eq> Eq for KeyWrapper<K> {}

/// LRU linked list entry.
struct LruEntry<K, V> {
	key: MaybeUninit<K>,
	val: MaybeUninit<V>,

	prev: Option<NonNull<LruEntry<K, V>>>,
	next: Option<NonNull<LruEntry<K, V>>>,
}

impl<K, V> LruEntry<K, V> {
	/// Moves the entry to the head.
	fn push_head(&mut self, head: &mut NonNull<Self>, tail: &mut NonNull<Self>) {
		let Some(mut prev) = self.prev.take() else {
			// The entry is already the head, do nothing
			return;
		};
		// If the entry is the tail, the previous entry becomes the new tail
		if self.next.is_none() {
			*tail = prev;
		}
		// Unlink entry
		unsafe {
			prev.as_mut().next = self.next;
			if let Some(mut next) = self.next {
				next.as_mut().prev = Some(prev);
			}
		}
		// Link to head
		self.next = Some(*head);
		unsafe {
			head.as_mut().prev = NonNull::new(self);
		}
		*head = NonNull::from(self);
	}

	/// Moves the entry to the tail.
	fn push_tail(&mut self, head: &mut NonNull<Self>, tail: &mut NonNull<Self>) {
		let Some(mut next) = self.next.take() else {
			// The entry is already the tail, do nothing
			return;
		};
		// If the entry is the head, the next entry becomes the new head
		if self.prev.is_none() {
			*head = next;
		}
		// Unlink entry
		unsafe {
			next.as_mut().prev = self.prev;
			if let Some(mut prev) = self.prev {
				prev.as_mut().next = Some(next);
			}
		}
		// Link to head
		self.prev = Some(*tail);
		unsafe {
			tail.as_mut().next = NonNull::new(self);
		}
		*tail = NonNull::from(self);
	}
}

/// Least Recently Used cache.
///
/// Contrary to the implementation from the `lru` crate, all the required memory is pre-allocated
/// at cache instantiation so that no failure can happen during operation.
pub struct LruCache<K: Eq + Hash, V> {
	/// Entries list.
	mem: NonNull<LruEntry<K, V>>,
	// use a HashSet instead of a HashMap to avoid storing the key twice
	/// Hash map to locate entries from key.
	hash: HashSet<KeyHash<K, V>>,

	// cannot be null since the capacity cannot be zero
	/// Most recent element.
	head: NonNull<LruEntry<K, V>>,
	/// Least recent element.
	tail: NonNull<LruEntry<K, V>>,
}

impl<K: Eq + Hash, V> LruCache<K, V> {
	/// Creates a new instance with the given capacity.
	pub fn new(capacity: NonZeroUsize) -> AllocResult<Self> {
		let cap = capacity.get();
		let hash = HashSet::with_capacity(cap)?;
		let layout = Layout::array::<LruEntry<K, V>>(cap).unwrap();
		unsafe {
			// Initialize entries
			let mem = __alloc(layout)?.cast();
			for i in 0..cap {
				mem.add(i).write(LruEntry {
					key: MaybeUninit::uninit(),
					val: MaybeUninit::uninit(),

					prev: i.checked_sub(1).map(|i| mem.add(i)),
					next: (i + 1 < cap).then_some(mem.add(i + 1)),
				});
			}
			Ok(Self {
				mem,
				hash,

				head: mem,
				tail: mem.add(cap - 1),
			})
		}
	}

	/// Returns the capacity of the cache.
	pub fn capacity(&self) -> usize {
		self.hash.capacity()
	}

	/// Returns the number of elements in the cache.
	pub fn len(&self) -> usize {
		self.hash.len()
	}

	/// Tells whether the cache is empty.
	pub fn is_empty(&self) -> bool {
		self.len() == 0
	}

	/// Returns an immutable reference to the value for the key `k`.
	///
	/// If not in cache, the function returns `None`.
	pub fn get<Q>(&mut self, k: &Q) -> Option<&V>
	where
		K: Borrow<Q>,
		Q: Hash + Eq + ?Sized,
	{
		self.hash
			.get(KeyWrapper::from_ref(k))
			.map(|ent| unsafe { ent.inner().val.assume_init_ref() })
	}

	/// Returns a mutable reference to the value for the key `k`.
	///
	/// If not in cache, the function returns `None`.
	pub fn get_mut<Q>(&mut self, k: &Q) -> Option<&mut V>
	where
		K: Borrow<Q>,
		Q: Hash + Eq + ?Sized,
	{
		self.hash
			.get(KeyWrapper::from_ref(k))
			.map(|ent| unsafe { ent.inner_mut().val.assume_init_mut() })
	}

	/// Pushes a key/value pair in the cache. If an entry with the key `k` already exists, it is
	/// removed and returned. Else, the function returns `None`.
	pub fn push(&mut self, k: K, v: V) -> Option<(K, V)> {
		// If an entry with the corresponding key already exists, just replace the key/value
		// and promote it
		if let Some(ent) = self.hash.get(KeyWrapper::from_ref(&k)) {
			unsafe {
				let ent = ent.inner_mut();
				let old = (
					mem::replace(ent.key.assume_init_mut(), k),
					mem::replace(ent.val.assume_init_mut(), v),
				);
				ent.push_head(&mut self.head, &mut self.tail);
				return Some(old);
			}
		}
		// If the cache is full, remove the tail element
		if self.len() == self.capacity() {
			let tail_key = unsafe { self.tail.as_ref().key.assume_init_ref() };
			// Cannot fail since all entries, and thus the tail, is used
			let ent = self.hash.remove(KeyWrapper::from_ref(tail_key)).unwrap();
			// Drop key/value before reusing the entry
			unsafe {
				let ent = ent.inner_mut();
				ent.key.assume_init_drop();
				ent.val.assume_init_drop();
			}
		}
		// Use tail as the new entry
		let ent = unsafe { self.tail.as_mut() };
		ent.key.write(k);
		ent.val.write(v);
		ent.push_head(&mut self.head, &mut self.tail);
		// Cannot fail since the memory required to insert the entry is already allocated
		self.hash
			.insert(KeyHash(self.head)) // the new entry is now the head
			.unwrap();
		None
	}

	/// Marks the key as the most recently used.
	pub fn promote<Q>(&mut self, k: &Q)
	where
		K: Borrow<Q>,
		Q: Hash + Eq + ?Sized,
	{
		let Some(ent) = self.hash.get(KeyWrapper::from_ref(k)) else {
			return;
		};
		let ent = unsafe { ent.inner_mut() };
		ent.push_head(&mut self.head, &mut self.tail);
	}

	/// Removes and returns the entry corresponding to the key `k`. If it does not exist, returns
	/// `None`.
	pub fn pop<Q>(&mut self, k: &Q) -> Option<V>
	where
		K: Borrow<Q>,
		Q: Hash + Eq + ?Sized,
	{
		let ent = self.hash.remove(KeyWrapper::from_ref(k))?;
		unsafe {
			let ent = ent.inner_mut();
			ent.push_tail(&mut self.head, &mut self.tail);
			// Drop key and retrieve value
			ent.key.assume_init_drop();
			Some(ent.val.assume_init_read())
		}
	}
}

impl<K: Eq + Hash, V> Drop for LruCache<K, V> {
	fn drop(&mut self) {
		let layout = Layout::array::<LruEntry<K, V>>(self.capacity()).unwrap();
		unsafe {
			__dealloc(self.mem.cast(), layout);
		}
	}
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn lru_simple() {
		let mut lru = LruCache::new(NonZeroUsize::new(10).unwrap()).unwrap();
		assert_eq!(lru.len(), 0);
		for i in 0..10 {
			lru.push(i, i);
		}
		assert_eq!(lru.len(), 10);
		for i in 0..10 {
			assert_eq!(lru.pop(&i), Some(i));
		}
		assert_eq!(lru.len(), 0);
	}

	#[test]
	fn lru_exhaust() {
		let mut lru = LruCache::new(NonZeroUsize::new(10).unwrap()).unwrap();
		assert_eq!(lru.len(), 0);
		for i in 0..100 {
			lru.push(i, i);
		}
		assert_eq!(lru.len(), lru.capacity());
		for i in 0..90 {
			assert_eq!(lru.pop(&i), None);
		}
		assert_eq!(lru.len(), lru.capacity());
		for i in (100 - lru.capacity())..100 {
			assert_eq!(lru.pop(&i), Some(i));
		}
		assert_eq!(lru.len(), 0);
	}
}
