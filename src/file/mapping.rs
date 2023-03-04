//! A file mapping is a view of a file in memory, which can be modified, shared between processes,
//! etc...

use core::ptr::NonNull;
use crate::errno::Errno;
use crate::file::FileLocation;
use crate::memory;
use crate::util::container::hashmap::HashMap;
use crate::util::container::map::Map;
use crate::util::math;

/// Structure representing a mapped page for a file.
pub struct Page {
	/// The pointer to the page.
	ptr: NonNull<[u8; memory::PAGE_SIZE]>,

	/// The number of references to the page.
	ref_count: u32,
}

/// A file mapped partially or totally into memory.
#[derive(Default)]
pub struct MappedFile {
	/// The list of mappings, ordered by offset in pages.
	pages: Map<usize, Page>,
}

impl MappedFile {
	/// Reads data from the mapped file and writes it into `buff`.
	///
	/// `off` is the offset in the mapped file to the beginning of the data to be read.
	///
	/// The function returns the number of read bytes.
	pub fn read(&mut self, off: usize, buff: &mut [u8]) -> usize {
		let pages_count = math::ceil_div(buff.len(), memory::PAGE_SIZE);
		let iter = self.pages.range(off..(off + pages_count));

		buff.fill(0);

		let len = 0;

		for (_mapping_off, _mapping) in iter {
			// TODO
			todo!();
		}

		len
	}

	/// Reads data from `buff` and writes it into the mapped file.
	///
	/// `off` is the offset in the mapped file to the beginning of the data to write.
	///
	/// On success, the function returns the number of written bytes.
	/// If the chunk of data is out of bounds on loaded mappings, the function returns None.
	pub fn write(&mut self, off: usize, buff: &[u8]) -> usize {
		let pages_count = math::ceil_div(buff.len(), memory::PAGE_SIZE);
		let iter = self.pages.range(off..(off + pages_count));

		let len = 0;

		for (_mapping_off, _mapping) in iter {
			// TODO
			todo!();
		}

		len
	}
}

/// Structure managing file mappings.
pub struct FileMappingManager {
	/// The list of mapped files, by location.
	mapped_files: HashMap<FileLocation, MappedFile>,
}

impl FileMappingManager {
	/// Creates a new instance.
	pub fn new() -> Self {
		Self {
			mapped_files: HashMap::new(),
		}
	}

	/// Returns a mutable reference to a mapped file.
	///
	/// If the file is not mapped, the function returns None.
	pub fn get_mapped_file(&mut self, loc: &FileLocation) -> Option<&mut MappedFile> {
		self.mapped_files.get_mut(loc)
	}

	/// Maps the the file at the given location.
	///
	/// Arguments:
	/// - `loc` is the location to the file.
	/// - `off` is the beginning offset of the chunk to map in pages.
	/// - `size` is the size of the chunk to map in pages.
	pub fn map(&mut self, loc: FileLocation, off: usize, len: usize) -> Result<(), Errno> {
		let mapped_file = match self.mapped_files.get_mut(&loc) {
			Some(f) => f,

			None => {
				self.mapped_files.insert(loc.clone(), MappedFile::default())?;
				self.mapped_files.get_mut(&loc).unwrap()
			},
		};

		// Increment references count
		mapped_file.pages.range_mut(off..(off + len))
			.for_each(|(_, p)| p.ref_count += 1);

		Ok(())
	}

	/// Unmaps the file at the given location.
	///
	/// Arguments:
	/// - `loc` is the location to the file.
	/// - `off` is the beginning offset of the chunk to map in pages.
	/// - `size` is the size of the chunk to map in pages.
	///
	/// If the file mapping doesn't exist, the function does nothing.
	pub fn unmap(&mut self, loc: &FileLocation, off: usize, len: usize) {
		let Some(mapped_file) = self.mapped_files.get_mut(loc) else {
			return;
		};

		// Decrement references count
		mapped_file.pages.range_mut(off..(off + len))
			.for_each(|(_, p)| p.ref_count -= 1);

		// Remove mapping that are not referenced
		mapped_file.pages.retain(|_, p| p.ref_count <= 0);

		// If no mapping is left for the file, remove it
		if mapped_file.pages.is_empty() {
			self.mapped_files.remove(loc);
		}
	}
}
