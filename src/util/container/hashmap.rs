//! A hashmap is a data structure that stores key/value pairs into buckets and uses the hash of the
//! key to quickly get the bucket storing the value.

use core::hash::Hash;
use core::hash::Hasher;
use core::mem::size_of_val;
use core::ops::Index;
use core::ops::IndexMut;
use core::ptr;
use crate::errno::Errno;
use super::vec::Vec;

/// The default number of buckets in a hashmap.
const DEFAULT_BUCKETS_COUNT: usize = 64;

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

impl<K: Eq + Hash, V> Bucket<K, V> {
	/// Creates a new instance.
	fn new() -> Self {
		Self {
			elements: Vec::new(),
		}
	}

	/// Returns an immutable reference to the value with the given key `k`. If the key isn't
	/// present, the function return None.
	pub fn get(&self, k: &K) -> Option<&V> {
		for i in 0..self.elements.len() {
			if self.elements[i].0 == *k {
				return Some(&self.elements[i].1);
			}
		}

		None
	}

	/// Returns a mutable reference to the value with the given key `k`. If the key isn't present,
	/// the function return None.
	pub fn get_mut(&mut self, k: &K) -> Option<&mut V> {
		for i in 0..self.elements.len() {
			if self.elements[i].0 == *k {
				return Some(&mut self.elements[i].1);
			}
		}

		None
	}

	/// Inserts a new element into the bucket. If the key was already present, the function
	/// returns the previous value.
	pub fn insert(&mut self, k: K, v: V) -> Result<Option<V>, Errno> {
		let old = self.remove(&k);
		self.elements.push((k, v))?;
		Ok(old)
	}

	/// Removes an element from the bucket. If the key was present, the function returns the
	/// value.
	pub fn remove(&mut self, k: &K) -> Option<V> {
		for i in 0..self.elements.len() {
			if self.elements[i].0 == *k {
				let val = unsafe {
					ptr::read(&self.elements[i].1 as _)
				};
				self.elements.remove(i);

				return Some(val);
			}
		}

		None
	}
}

/// Structure representing a hashmap.
pub struct HashMap<K: Eq + Hash, V> {
	/// The number of buckets in the hashmap.
	buckets_count: usize,

	/// The vector containing buckets.
	buckets: Vec<Bucket<K, V>>,
}

impl<K: Eq + Hash, V> HashMap::<K, V> {
	/// Creates a new instance with the default number of buckets.
	pub const fn new() -> Self {
		Self {
			buckets_count: DEFAULT_BUCKETS_COUNT,

			buckets: Vec::new(),
		}
	}

	/// Creates a new instance with the given number of buckets.
	pub const fn with_buckets(buckets_count: usize) -> Self {
		Self {
			buckets_count,

			buckets: Vec::new(),
		}
	}

	/// Returns the number of elements in the hash map.
	pub fn len(&self) -> usize {
		let mut total = 0;

		for b in self.buckets.iter() {
			total += b.elements.len();
		}

		total
	}

	/// Tells whether the hash map is empty.
	pub fn is_empty(&self) -> bool {
		self.len() == 0
	}

	/// Returns the number of buckets.
	pub fn get_buckets_count(&self) -> usize {
		self.buckets_count
	}

	/// Returns the bucket index for the key `k`.
	fn get_bucket_index(&self, k: &K) -> usize {
		let mut hasher = XORHasher::new();
		k.hash(&mut hasher);
		(hasher.finish() / (self.buckets_count as u64)) as usize
	}

	/// Returns an immutable reference to the value with the given key `k`. If the key isn't
	/// present, the function return None.
	pub fn get(&self, k: &K) -> Option<&V> {
		let index = self.get_bucket_index(&k);

		if index < self.buckets.len() {
			self.buckets[index].get(k)
		} else {
			None
		}
	}

	/// Returns a mutable reference to the value with the given key `k`. If the key isn't present,
	/// the function return None.
	pub fn get_mut(&mut self, k: &K) -> Option<&mut V> {
		let index = self.get_bucket_index(&k);

		if index < self.buckets.len() {
			self.buckets[index].get_mut(k)
		} else {
			None
		}
	}

	/// Creates an iterator for the hash map.
	pub fn iter(&self) -> HashMapIterator<K, V> {
		HashMapIterator::new(self)
	}

	/// Inserts a new element into the hash map. If the key was already present, the function
	/// returns the previous value.
	pub fn insert(&mut self, k: K, v: V) -> Result<Option<V>, Errno> {
		let index = self.get_bucket_index(&k);
		if index >= self.buckets.len() {
			// Creating buckets
			let begin = self.buckets.len();
			for i in begin..=index {
				self.buckets.insert(i, Bucket::new())?;
			}
		}

		self.buckets[index].insert(k, v)
	}

	/// Removes an element from the hash map. If the key was present, the function returns the
	/// value.
	pub fn remove(&mut self, k: K) -> Option<V> {
		let index = self.get_bucket_index(&k);

		if index < self.buckets.len() {
			self.buckets[index].remove(&k)
		} else {
			None
		}
	}

	/// Drops all elements in the hash map.
	pub fn clear(&mut self) {
		for i in 0..self.buckets.len() {
			self.buckets[i].elements.clear();
		}
	}
}

impl<K: Eq + Hash, V> Index<K> for HashMap<K, V> {
	type Output = V;

	#[inline]
	fn index(&self, k: K) -> &Self::Output {
		self.get(&k).expect("no entry found for key")
	}
}

impl<K: Eq + Hash, V> IndexMut<K> for HashMap<K, V> {
	#[inline]
	fn index_mut(&mut self, k: K) -> &mut Self::Output {
		self.get_mut(&k).expect("no entry found for key")
	}
}

/// An iterator for the Vec structure.
pub struct HashMapIterator<'a, K: Hash + Eq, V> {
	/// The hash map to iterate into.
	hm: &'a HashMap<K, V>,

	/// The current bucket index.
	curr_bucket: usize,
	/// The current element index.
	curr_element: usize,
}

impl<'a, K: Hash + Eq, V> HashMapIterator<'a, K, V> {
	/// Creates a hash map iterator for the given reference.
	fn new(hm: &'a HashMap<K, V>) -> Self {
		Self {
			hm,

			curr_bucket: 0,
			curr_element: 0,
		}
	}
}

impl<'a, K: Hash + Eq, V> Iterator for HashMapIterator<'a, K, V> {
	type Item = &'a V;

	fn next(&mut self) -> Option<Self::Item> {
		if self.curr_bucket >= self.hm.buckets.len() {
			return None;
		}

		// If the last element has been reached, getting the next non-empty bucket
		if self.curr_element >= self.hm.buckets[self.curr_bucket].elements.len() {
			self.curr_element = 0;
			self.curr_bucket += 1;

			for i in self.curr_bucket..self.hm.buckets.len() {
				if !self.hm.buckets[i].elements.is_empty() {
					self.curr_bucket += i;
					break;
				}
			}

			if self.curr_bucket >= self.hm.buckets.len() {
				return None
			}
		}

		let e = &self.hm.buckets[self.curr_bucket].elements[self.curr_element].1;
		self.curr_element += 1;
		Some(e)
	}

	fn count(self) -> usize {
		self.hm.len()
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

		assert_eq!(hash_map.remove(0).unwrap(), 0);

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
			assert_eq!(hash_map.remove(i as _).unwrap(), 0);
			assert_eq!(hash_map.len(), i);
		}
	}
}
