//! This module implements storage caches, allowing to cache disk sectors to
//! avoid unnecessary accesses to the storage device.

use crate::device::storage::StorageInterface;
use crate::errno::Errno;
use crate::memory::malloc;
use crate::util::boxed::Box;
use crate::util::container::hashmap::HashMap;
use crate::util::container::ring_buffer::RingBuffer;
use crate::util::container::vec::Vec;

/// Structure representing a cached sector.
struct CachedSector {
	/// Tells whether the sector has been written.
	written: bool,

	/// The sector's data.
	data: malloc::Alloc<u8>,
}

/// Structure representing a storage cache. The number of sectors and their size
/// is fixed at initialization.
/// The cache needs to be flushed before being dropped. Otherwise, the updated
/// data shall be lost.
pub struct StorageCache {
	/// The size of a sector in bytes.
	sector_size: usize,

	/// Cached sectors.
	sectors: HashMap<u64, CachedSector>,

	/// Fifo storing sector indexes. When the fifo is full, the oldest sector
	/// shall be discarded from the cache.
	fifo: RingBuffer<u64, Vec<u64>>,
}

impl StorageCache {
	/// Creates a new instance.
	/// `count` is maximum number of sectors the cache can contain.
	/// `size` is the size of a sector in bytes.
	pub fn new(count: usize, size: usize) -> Result<Self, Errno> {
		Ok(Self {
			sector_size: size,

			sectors: HashMap::new(),

			fifo: RingBuffer::new(crate::vec![0; count]?),
		})
	}

	/// Tells whether the cache is full.
	pub fn is_full(&self) -> bool {
		self.sectors.len() >= self.fifo.get_size()
	}

	/// Reads the sector with index `sector` and writes its content into the
	/// buffer `buff`.
	pub fn read(&mut self, sector: u64, buff: &mut [u8]) -> Result<Option<()>, Errno> {
		if let Some(CachedSector {
			data, ..
		}) = self.sectors.get(&sector)
		{
			buff.copy_from_slice(data.as_slice());
			Ok(Some(()))
		} else {
			Ok(None)
		}
	}

	/// Writes the sector with index `sector` with the content of the buffer
	/// `buff`. `flush_hook` is a function used to write a sector to the disk.
	/// It is called in case the cache needs to free up a slot for a sector.
	pub fn write(&mut self, sector: u64, buff: &[u8]) -> Result<Option<()>, Errno> {
		if let Some(CachedSector {
			written,
			data,
			..
		}) = self.sectors.get_mut(&sector)
		{
			data.as_slice_mut().copy_from_slice(buff);
			*written = true;
			Ok(Some(()))
		} else {
			Ok(None)
		}
	}

	/// Inserts a new sector in the cache. If no space is left, the cache frees
	/// up slots of other sectors to retrieve space.
	/// Sectors that have been freed up are written to the disk using
	/// `flush_hook`. `sector` is the index of the sector.
	/// `buff` is a buffer containing the sector's data.
	pub fn insert<F>(&mut self, sector: u64, buff: &[u8], mut flush_hook: F) -> Result<(), Errno>
	where
		F: FnMut(u64, &[u8]) -> Result<(), Errno>,
	{
		// Freeing some slots if needed
		if self.is_full() {
			let mut sector_indexes: [u64; 16] = Default::default();
			let n = self.fifo.read(&mut sector_indexes);

			for i in &sector_indexes[..n] {
				let sector = self.sectors.remove(i).unwrap();
				if sector.written {
					flush_hook(*i, sector.data.as_slice())?;
				}
			}
		}

		let mut alloc = malloc::Alloc::<u8>::new_default(self.sector_size)?;
		alloc.as_slice_mut().copy_from_slice(buff);

		self.sectors.insert(
			sector,
			CachedSector {
				written: false,

				data: alloc,
			},
		)?;
		self.fifo.write(&[sector]);

		Ok(())
	}

	/// Flushes the cache, writing every updated sectors to the disk.
	/// Sectors that have been updated are written to the disk using
	/// `flush_hook`. On error, flushing is not completed.
	pub fn flush<F>(&mut self, mut flush_hook: F) -> Result<(), Errno>
	where
		F: FnMut(u64, &[u8]) -> Result<(), Errno>,
	{
		// Writing updated sectors
		for (index, sector) in self.sectors.iter() {
			if sector.written {
				flush_hook(*index, sector.data.as_slice())?;
			}
		}

		self.sectors.clear();
		self.fifo.clear();
		Ok(())
	}
}

/// Structure representing a storage interface wrapped into a cache.
/// On drop, the cache is flushed to the storage device. When flushing fails,
/// data is lost.
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
	pub fn new(
		storage_interface: Box<dyn StorageInterface>,
		sectors_count: usize,
	) -> Result<Self, Errno> {
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

	fn read(&mut self, buf: &mut [u8], offset: u64, size: u64) -> Result<(), Errno> {
		let block_size = self.get_block_size();

		for i in 0..size {
			let buf_begin = (block_size * i) as usize;
			let buf_end = (block_size * (i + 1)) as usize;
			let buf = &mut buf[buf_begin..buf_end];

			if self.cache.read(offset + i, buf)?.is_none() {
				self.storage_interface.read(buf, offset + i, 1)?;

				self.cache.insert(offset + i, buf, |off, buf| {
					self.storage_interface.write(buf, off, 1)
				})?;
			}
		}

		Ok(())
	}

	fn write(&mut self, buf: &[u8], offset: u64, size: u64) -> Result<(), Errno> {
		let block_size = self.get_block_size();

		for i in 0..size {
			let buf_begin = (block_size * i) as usize;
			let buf_end = (block_size * (i + 1)) as usize;
			let buf = &buf[buf_begin..buf_end];

			if self.cache.write(offset + i, buf)?.is_none() {
				self.cache.insert(offset + i, buf, |off, buf| {
					self.storage_interface.write(buf, off, 1)
				})?;
			}
		}

		Ok(())
	}
}

impl Drop for CachedStorageInterface {
	fn drop(&mut self) {
		let _ = self.cache.flush(|off, buf| {
			let _ = self.storage_interface.write(buf, off, 1);
			Ok(())
		});
	}
}
