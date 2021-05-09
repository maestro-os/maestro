/// A hashmap is a data structure that stores key/value pairs into buckets and uses the hash of the key to quickly get the bucket storing the value.

use core::hash::Hash;
use core::hash::Hasher;
use core::mem::size_of_val;
use core::ops::Index;
use core::ops::IndexMut;
use super::vec::Vec;

/// Bitwise XOR hasher.
struct XORHasher {
	/// The currently stored value.
	value: u64,
	/// The offset byte at which the next XOR operation shall be performed.
	off: u8,
}

impl XORHasher {
	/// Creates a new instance.
	pub fn new() -> Self {
		Self {
			value: 0,
			off: 0,
		}
	}
}

impl Hasher for XORHasher {
	fn write(&mut self, bytes: &[u8]) {
		for b in bytes {
			self.value ^= (*b as u64) << (self.off * 8);
			self.off = (self.off + 1) % size_of_val(&self.value) as u8;
		}
	}

	fn finish(&self) -> u64 {
		self.value
	}
}

/// A bucket is a list storing elements that match a given hash range.
/// Since hashing function have collisions, several elements can have the same hash.
struct Bucket<K: Eq + Hash, V> {
	/// The vector storing the key/value pairs.
	elements: Vec<(K, V)>,
}

/// Structure representing a hashmap.
pub struct HashMap<K: Eq + Hash, V> {
	/// The vector containing buckets.
	buckets: Vec<Bucket<K, V>>,
}

impl<K: Eq + Hash, V> HashMap::<K, V> {
	/// Creates a new instance.
	pub fn new() -> Self {
		Self {
			buckets: Vec::new(),
		}
	}

	/// Returns an immutable reference to the value with the given key `k`. If the key isn't
	/// present, the function return None.
	pub fn get(&self, _k: K) -> Option<&V> {
		// TODO
		None
	}

	/// Returns a mutable reference to the value with the given key `k`. If the key isn't present,
	/// the function return None.
	pub fn get_mut(&mut self, _k: K) -> Option<&mut V> {
		// TODO
		None
	}

	/// Inserts a new element into the hash map. If the key was already present, the function
	/// returns the previous value.
	pub fn insert(&mut self, _k: K, _v: V) -> Option<V> {
		// TODO
		None
	}

	/// Removes an element from the hash map. If the key was present, the function returns the
	/// value.
	pub fn remove(&mut self, _k: K, _v: V) -> Option<V> {
		// TODO
		None
	}

	/// Drops all elements in the hash map.
	pub fn clear(&mut self) {
		// TODO
	}
}

impl<K: Eq + Hash, V> Index<K> for HashMap<K, V> {
	type Output = V;

	#[inline]
	fn index(&self, k: K) -> &Self::Output {
		self.get(k).expect("no entry found for key")
	}
}

impl<K: Eq + Hash, V> IndexMut<K> for HashMap<K, V> {
	#[inline]
	fn index_mut(&mut self, k: K) -> &mut Self::Output {
		self.get_mut(k).expect("no entry found for key")
	}
}

// TODO Iterator

#[cfg(test)]
mod test {
	use super::*;

	#[test_case]
	fn hashmap0() {
		// TODO
	}
}
