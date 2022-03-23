//! This module implements storage caches, allowing to cache disk sectors to avoid unnecessary
//! accesses to the storage device.

use core::cmp::max;
use core::intrinsics::wrapping_add;
use crate::errno::Errno;
use crate::memory::malloc;
use crate::util::container::hashmap::HashMap;
use crate::util::container::vec::Vec;

/// Structure representing a cached sector.
struct CachedSector {
	/// Tells whether the sector has been written.
	written: bool,

	/// The sector's data.
	data: malloc::Alloc<u8>,
}

/// Structure representing a storage cache. The number of sectors and their size is fixed at
/// initialization.
/// The cache needs to be flushed before being dropped. Otherwise, the kernel shall panic.
pub struct StorageCache {
	/// The maximum number of sectors the cache can contain.
	sectors_count: usize,
	/// The size of a sector in bytes.
	sector_size: usize,

	/// Cached sectors.
	sectors: HashMap<u64, CachedSector>,

	/// Vector storing index of each sectors and their accesses count, stored by decreasing
	/// accesses count.
	access_stats: Vec<(u64, u32)>,
}

impl StorageCache {
	/// Creates a new instance.
	/// `count` is maximum number of sectors the cache can contain.
	/// `size` is the size of a sector in bytes.
	pub fn new(count: usize, size: usize) -> Self {
		Self {
			sectors_count: count,
			sector_size: size,

			sectors: HashMap::new(),

			access_stats: Vec::new(),
		}
	}

	/// Tells whether the cache is full.
	pub fn is_full(&self) -> bool {
		self.sectors.len() >= self.sectors_count
	}

	/// Increments the access count for the given sector `sector`.
	fn increment_access(&mut self, sector: u64) -> Result<(), Errno> {
		match self.access_stats.binary_search_by(| e | e.0.cmp(&sector).reverse()) {
			Ok(n) => {
				let (sector, access_count) = self.access_stats[n];

				// On overflow, the value becomes zero. This behaviour increases the odds of the
				// sector to be freed from the cache even though it is often accessed, but it is
				// acceptable when taking into account the size of the counter
				self.access_stats[n] = (sector, wrapping_add(access_count, 1));
			},

			Err(n) => {
				self.access_stats.insert(n, (sector, 1))?;
			},
		}

		Ok(())
	}

	/// Reads the sector with index `sector` and writes its content into the buffer `buff`.
	pub fn read(&mut self, sector: u64, buff: &mut [u8]) -> Result<Option<()>, Errno> {
		self.increment_access(sector)?;

		// Updating data
		if let Some(CachedSector { written: _, data }) = self.sectors.get(&sector) {
			buff.copy_from_slice(data.get_slice());
			Ok(Some(()))
		} else {
			Ok(None)
		}
	}

	/// Writes the sector with index `sector` with the content of the buffer `buff`.
	/// `flush_hook` is a function used to write a sector to the disk. It is called in case the
	/// cache needs to free up a slot for a sector.
	pub fn write(&mut self, sector: u64, buff: &[u8]) -> Result<Option<()>, Errno> {
		self.increment_access(sector)?;

		// Updating data
		if let Some(CachedSector { written, data }) = self.sectors.get_mut(&sector) {
			data.get_slice_mut().copy_from_slice(buff);
			*written = true;

			Ok(Some(()))
		} else {
			Ok(None)
		}
	}

	/// Inserts a new sector in the cache. If no space is left, the cache frees up slots of other
	/// sectors to retrieve space.
	/// Sectors that have been freed up are written to the disk using `flush_hook`.
	/// `sector` is the index of the sector.
	/// `buff` is a buffer containing the sector's data.
	pub fn insert<F>(&mut self, sector: u64, buff: &[u8], flush_hook: F) -> Result<(), Errno>
		where F: Fn(u64, &[u8]) -> Result<(), Errno> {
		if self.is_full() {
			// Freeing some slots

			let free_count = 0; // TODO Find an heuristic to compute the most optimal value
			for _ in 0..max(free_count, 1) {
				if let Some((sector_index, _)) = self.access_stats.pop() {
					let sector = self.sectors.get(&sector_index).unwrap();
					flush_hook(sector_index, sector.data.get_slice())?;

					self.sectors.remove(&sector_index);
				}
			}
		}

		let mut alloc = malloc::Alloc::<u8>::new_default(self.sector_size)?;
		alloc.get_slice_mut().copy_from_slice(buff);

		self.sectors.insert(sector, CachedSector {
			written: false,
			data: alloc,
		})?;
		Ok(())
	}

	/// Flushes the cache, writing every updated sectors to the disk.
	/// Sectors that have been updated are written to the disk using `flush_hook`.
	/// Flusing is mandatory before dropping the cache, otherwise, the kernel shall panic.
	/// On error, flushing is not completed.
	pub fn flush<F>(&mut self, flush_hook: F) -> Result<(), Errno>
		where F: Fn(u64, &[u8]) -> Result<(), Errno> {
		// Writing updated sectors
		for (index, sector) in self.sectors.iter() {
			if sector.written {
				flush_hook(*index, sector.data.get_slice())?;
			}
		}

		self.discard();
		Ok(())
	}

	/// Discards the cache's data, allowing to drop it.
	/// This function should be used only when flushing failed and the cache needs to be dropped
	/// anyways.
	pub fn discard(&mut self) {
		self.sectors.clear();
		self.access_stats.clear();
	}
}

impl Drop for StorageCache {
	fn drop(&mut self) {
		if !self.sectors.is_empty() {
			crate::kernel_panic!("Internal error: freeing a storage cache requires flushing it \
first");
		}
	}
}
