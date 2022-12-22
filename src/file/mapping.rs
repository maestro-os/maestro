//! A file mapping is a view of a file in memory, which can be modified, shared between processes,
//! etc...

use core::cmp::Ordering;
use core::cmp::min;
use core::ptr::NonNull;
use core::slice;
use crate::errno::Errno;
use crate::file::FileLocation;
use crate::memory;
use crate::util::container::hashmap::HashMap;
use crate::util::container::map::Map;
use crate::util::container::vec::Vec;
use crate::util::math;

/// A mapping on a file.
struct FileMapping {
	/// The offset to the beginning of the mapping in bytes.
	off: u64,
	/// The length of the mapping in number of pages.
	len: usize,

	/// The list of physical pages of the mapping.
	pages: Vec<NonNull<u8>>,
}

impl FileMapping {
	/// TODO doc
	pub fn read(&self, off: usize, buff: &mut [u8]) -> usize {
		// The total length in bytes
		let total_len = self.pages.len() * memory::PAGE_SIZE;

		let end = min(off + buff.len(), total_len);

		let begin_page = off / memory::PAGE_SIZE;
		let end_page = math::ceil_division(end, memory::PAGE_SIZE);

		self.pages[begin_page..end_page]
			.iter()
			.enumerate()
			.map(|(i, page)| (i * memory::PAGE_SIZE, page))
			.map(|(page_off, page)| {
				let len = min(page_off + memory::PAGE_SIZE, end);

				let page_slice = unsafe {
					slice::from_raw_parts(page.as_ref(), len)
				};
				buff[page_off..(page_off + len)].copy_from_slice(&page_slice);

				len
			})
			.sum()
	}

	/// TODO doc
	pub fn write(&mut self, off: usize, buff: &[u8]) -> usize {
		// The total length in bytes
		let total_len = self.pages.len() * memory::PAGE_SIZE;

		let end = min(off + buff.len(), total_len);

		let begin_page = off / memory::PAGE_SIZE;
		let end_page = math::ceil_division(end, memory::PAGE_SIZE);

		self.pages[begin_page..end_page]
			.iter_mut()
			.enumerate()
			.map(|(i, page)| (i * memory::PAGE_SIZE, page))
			.map(|(page_off, page)| {
				let len = min(page_off + memory::PAGE_SIZE, end);

				let page_slice = unsafe {
					slice::from_raw_parts_mut(page.as_mut(), len)
				};
				page_slice.copy_from_slice(&buff[page_off..(page_off + len)]);

				len
			})
			.sum()
	}
}

/// A file mapped partially or totally into memory.
#[derive(Default)]
pub struct MappedFile {
	/// The list of mappings, ordered by offset.
	mappings: Map<u64, FileMapping>,
}

impl MappedFile {
	/// Returns the mapping corresponding to the given offset `off`.
	///
	/// If the given offset doesn't match any mapping, the function returns None.
	fn get_mapping_for(&mut self, off: u64) -> Option<&mut FileMapping> {
		self.mappings.cmp_get_mut(|key, value| {
			let begin = *key;
			let end = begin as u64 + value.len as u64 * memory::PAGE_SIZE as u64;

			if off >= begin && off < end {
				Ordering::Equal
			} else if off < begin {
				Ordering::Less
			} else {
				Ordering::Greater
			}
		})
	}

	/// Reads data from the mapped file and writes it into `buff`.
	///
	/// `off` is the offset in the mapped file to the beginning of the data to be read.
	///
	/// The function returns the number of read bytes.
	pub fn read(&mut self, off: u64, buff: &mut [u8]) -> usize {
		let mut i = 0;

		while i < buff.len() {
			let off = off + i as u64;
			let Some(mapping) = self.get_mapping_for(off) else {
				break;
			};

			i += mapping.read((off - i as u64) as usize, &mut buff[i..]);
		}

		i
	}

	/// Reads data from `buff` and writes it into the mapped file.
	///
	/// `off` is the offset in the mapped file to the beginning of the data to write.
	///
	/// On success, the function returns the number of written bytes.
	/// If the chunk of data is out of bounds on loaded mappings, the function returns None.
	pub fn write(&mut self, off: u64, buff: &[u8]) -> usize {
		let mut i = 0;

		while i < buff.len() {
			let off = off + i as u64;
			let Some(mapping) = self.get_mapping_for(off) else {
				break;
			};

			i += mapping.write((off - i as u64) as usize, &buff[i..]);
		}

		i
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
	/// - `off` is the beginning offset of the chunk to map.
	/// - `size` is the size of the chunk to map in pages.
	pub fn map(&mut self, loc: FileLocation, off: u64, len: usize) -> Result<(), Errno> {
		let mapped_file = match self.mapped_files.get_mut(&loc) {
			Some(f) => f,

			None => {
				self.mapped_files.insert(loc.clone(), MappedFile::default())?;
				self.mapped_files.get_mut(&loc).unwrap()
			},
		};

		let mut i = 0;
		while i < len {
			match mapped_file.get_mapping_for(off) {
				Some(mapping) => {
					// TODO increment references count and return

					i += mapping.len;
				},

				None => {
					// No mapping match. Create one
					// TODO Handle overlapping
					let mapping_off = off + i as u64 * memory::PAGE_SIZE as u64;
					mapped_file.mappings.insert(mapping_off, FileMapping {
						off: mapping_off,
						len: len - i,

						pages: Vec::new(),
					})?;

					break;
				}
			}
		}

		Ok(())
	}

	/// Unmaps the file at the given location.
	///
	/// Arguments:
	/// - `loc` is the location to the file.
	/// - `off` is the beginning offset of the chunk to map.
	/// - `size` is the size of the chunk to map in pages.
	///
	/// If the file mapping doesn't exist, the function does nothing.
	pub fn unmap(&mut self, loc: &FileLocation, _off: u64, _len: usize) {
		let Some(mapped_file) = self.mapped_files.get(loc) else {
			return;
		};

		// TODO remove mapping(s)

		// If no mapping is left for the file, remove it
		if mapped_file.mappings.is_empty() {
			self.mapped_files.remove(loc);
		}
	}
}
