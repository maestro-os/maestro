//! This module implements storage caches, allowing to cache disk sectors to avoid unnecessary
//! accesses to the storage device.

use core::cmp::max;
use core::intrinsics::wrapping_add;
use crate::device::storage::StorageInterface;
use crate::errno::Errno;
use crate::memory::malloc;
use crate::util::boxed::Box;
use crate::util::container::hashmap::HashMap;
use crate::util::container::vec::Vec;

/// Structure representing a cached sector.
struct CachedSector {
	/// Tells whether the sector has been written.
	written: bool,
	/// The number of times the sector has been accessed.
	access_count: u32,

	/// The sector's data.
	data: malloc::Alloc<u8>,
}

/// Structure representing a storage cache. The number of sectors and their size is fixed at
/// initialization.
/// The cache needs to be flushed before being dropped. Otherwise, the updated data shall be lost.
pub struct StorageCache {
	/// The size of a sector in bytes.
	sector_size: usize,

	/// Cached sectors.
	sectors: HashMap<u64, CachedSector>,

	/// Vector storing index of each sectors, stored by decreasing accesses count.
	access_stats: Vec<u64>,
}

impl StorageCache {
	/// Creates a new instance.
	/// `count` is maximum number of sectors the cache can contain.
	/// `size` is the size of a sector in bytes.
	pub fn new(count: usize, size: usize) -> Result<Self, Errno> {
		Ok(Self {
			sector_size: size,

			sectors: HashMap::new(),

			access_stats: Vec::with_capacity(count)?,
		})
	}

	/// Tells whether the cache is full.
	pub fn is_full(&self) -> bool {
		self.sectors.len() >= self.access_stats.capacity()
	}

	// TODO Rewrite (doesn't work)
	/// Increments the access count for the given sector `sector`.
	fn increment_access(&mut self, sector: u64) -> Result<(), Errno> {
		let old_access_count = self.sectors.get(&sector).unwrap().access_count;

		// TODO Remove from access stats

		// On overflow, the value becomes zero. This behaviour increases the odds of the
		// sector to be freed from the cache even though it is often accessed, but it is
		// acceptable when taking into account the size of the counter
		let access_count = wrapping_add(old_access_count, 1);

		let n = self.access_stats.binary_search_by(| e | e.cmp(&(access_count as _)).reverse())
			.unwrap_or_else(| n | n);
		self.access_stats.insert(n, sector)?;

		Ok(())
	}

	/// Reads the sector with index `sector` and writes its content into the buffer `buff`.
	pub fn read(&mut self, sector: u64, buff: &mut [u8]) -> Result<Option<()>, Errno> {
		if let Some(CachedSector { data, .. }) = self.sectors.get(&sector) {
			buff.copy_from_slice(data.get_slice());

			self.increment_access(sector)?;
			Ok(Some(()))
		} else {
			Ok(None)
		}
	}

	/// Writes the sector with index `sector` with the content of the buffer `buff`.
	/// `flush_hook` is a function used to write a sector to the disk. It is called in case the
	/// cache needs to free up a slot for a sector.
	pub fn write(&mut self, sector: u64, buff: &[u8]) -> Result<Option<()>, Errno> {
		if let Some(CachedSector { written, data, .. }) = self.sectors.get_mut(&sector) {
			data.get_slice_mut().copy_from_slice(buff);
			*written = true;

			self.increment_access(sector)?;
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
	pub fn insert<F>(&mut self, sector: u64, buff: &[u8], mut flush_hook: F) -> Result<(), Errno>
		where F: FnMut(u64, &[u8]) -> Result<(), Errno> {
		if self.access_stats.capacity() == 0 {
			return Err(crate::errno!(ENOMEM));
		}

		// Freeing some slots if needed
		if self.is_full() {
			let free_count = 0; // TODO Find an heuristic to compute the most optimal value
			for _ in 0..max(free_count, 1) {
				let sector_index = self.access_stats[self.access_stats.len() - 1];

				let sector = self.sectors.get(&sector_index).unwrap();
				if sector.written {
					flush_hook(sector_index, sector.data.get_slice())?;
				}

				self.sectors.remove(&sector_index);
				self.access_stats.pop();
			}
		}

		let mut alloc = malloc::Alloc::<u8>::new_default(self.sector_size)?;
		alloc.get_slice_mut().copy_from_slice(buff);

		self.sectors.insert(sector, CachedSector {
			written: false,
			access_count: 1,

			data: alloc,
		})?;
		self.access_stats.push(sector)?;
		Ok(())
	}

	/// Flushes the cache, writing every updated sectors to the disk.
	/// Sectors that have been updated are written to the disk using `flush_hook`.
	/// On error, flushing is not completed.
	pub fn flush<F>(&mut self, mut flush_hook: F) -> Result<(), Errno>
		where F: FnMut(u64, &[u8]) -> Result<(), Errno> {
		// Writing updated sectors
		for (index, sector) in self.sectors.iter() {
			if sector.written {
				flush_hook(*index, sector.data.get_slice())?;
			}
		}

		self.sectors.clear();
		self.access_stats.clear();
		Ok(())
	}
}

/// Structure representing a storage interface wrapped into a cache.
/// On drop, the cache is flushed to the storage device. When flushing fails, data is lost.
pub struct CachedStorageInterface {
	/// The wrapped interface.
	storage_interface: Box<dyn StorageInterface>,

	/// The cache.
	cache: StorageCache,
}

impl CachedStorageInterface {
	/// Creates a new instance.
	/// `storage_interface` is the interface to wrap.
	/// `sectors_count` is the maximum number of sectors in the cache.
	pub fn new(storage_interface: Box<dyn StorageInterface>, sectors_count: usize)
		-> Result<Self, Errno> {
		let sector_size = storage_interface.get_block_size() as _;

		Ok(Self {
			storage_interface,

			cache: StorageCache::new(sectors_count, sector_size)?,
		})
	}
}

impl StorageInterface for CachedStorageInterface {
	fn get_block_size(&self) -> u64 {
		self.storage_interface.get_block_size()
	}

	fn get_blocks_count(&self) -> u64 {
		self.storage_interface.get_blocks_count()
	}

	// TODO Optimize to allow calling the storage interface for several blocks at once
	fn read(&mut self, buf: &mut [u8], offset: u64, size: u64) -> Result<(), Errno> {
		let block_size = self.get_block_size();

		for i in 0..size {
			let buf_begin = (block_size * i) as usize;
			let buf_end = (block_size * (i + 1)) as usize;
			let buf = &mut buf[buf_begin..buf_end];

			if self.cache.read(offset + i, buf)?.is_none() {
				self.storage_interface.read(buf, offset + i, 1)?;

				self.cache.insert(offset + i, buf, | off, buf | {
					self.storage_interface.write(buf, off, 1)
				})?;
			}
		}

		Ok(())
	}

	// TODO Optimize to allow calling the storage interface for several blocks at once
	fn write(&mut self, buf: &[u8], offset: u64, size: u64) -> Result<(), Errno> {
		let block_size = self.get_block_size();

		for i in 0..size {
			let buf_begin = (block_size * i) as usize;
			let buf_end = (block_size * (i + 1)) as usize;
			let buf = &buf[buf_begin..buf_end];

			if self.cache.write(offset + i, buf)?.is_none() {
				self.cache.insert(offset + i, buf, | off, buf | {
					self.storage_interface.write(buf, off, 1)
				})?;
			}
		}

		Ok(())

	}
}

impl Drop for CachedStorageInterface {
	fn drop(&mut self) {
		let _ = self.cache.flush(| off, buf | {
			let _ = self.storage_interface.write(buf, off, 1);
			Ok(())
		});
	}
}
