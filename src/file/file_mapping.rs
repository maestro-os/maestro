//! A file mapping is a view of a file in memory, which can be modified, shared between processes,
//! etc...

use core::cmp::Ordering;
use core::ptr::NonNull;
use core::slice;
use crate::file::FileLocation;
use crate::memory;
use crate::util::container::hashmap::HashMap;
use crate::util::container::map::Map;

/// A mapping on a file.
struct FileMapping {
	/// The offset to the beginning of the mapping in bytes.
	begin: u64,
	/// The length of the mapping in number of pages.
	len: usize,

	/// The content of the mapping.
	content: NonNull<u8>,
}

/// A file mapped partially or totally into memory.
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
	/// `offset` is the offset in the mapped file to the beginning of the data to be read.
	///
	/// On success, the function returns the number of read bytes.
	/// If the chunk of data is out of bounds on loaded mappings, the function returns None.
	pub fn read(&mut self, offset: u64, buff: &mut [u8]) -> Option<u64> {
		let mapping = self.get_mapping_for(offset)?;
		let mapping_len = mapping.len * memory::PAGE_SIZE;

		// The begin offset inside of the mapping
		let begin = offset - mapping.begin;
		// The end offset inside of the mapping
		let end = begin + buff.len() as u64;

		if end <= mapping_len as u64 {
			let content_slice = unsafe {
				slice::from_raw_parts(mapping.content.as_mut(), mapping_len)
			};

			buff.copy_from_slice(&content_slice[(begin as usize)..(end as usize)]);
			Some(end - begin)
		} else {
			None
		}
	}

	/// Reads data from `buff` and writes it into the mapped file.
	///
	/// `offset` is the offset in the mapped file to the beginning of the data to write.
	///
	/// On success, the function returns the number of written bytes.
	/// If the chunk of data is out of bounds on loaded mappings, the function returns None.
	pub fn write(&mut self, offset: u64, buff: &[u8]) -> Option<u64> {
		let mapping = self.get_mapping_for(offset)?;
		let mapping_len = mapping.len * memory::PAGE_SIZE;

		// The begin offset inside of the mapping
		let begin = offset - mapping.begin;
		// The end offset inside of the mapping
		let end = begin + buff.len() as u64;

		if end <= mapping_len as u64 {
			let content_slice = unsafe {
				slice::from_raw_parts_mut(mapping.content.as_mut(), mapping_len)
			};

			content_slice[(begin as usize)..(end as usize)].copy_from_slice(&buff);
			Some(end - begin)
		} else {
			None
		}
	}
}

/// Structure managing file mappings.
pub struct FileMappingManager {
	/// The list of mapped files, by location.
	file_mappings: HashMap<FileLocation, MappedFile>,
}

impl FileMappingManager {
	/// Creates a new instance.
	pub fn new() -> Self {
		Self {
			file_mappings: HashMap::new(),
		}
	}

	/// Returns a mutable reference to a mapped file.
	///
	/// If the file is not mapped, the function returns None.
	pub fn get_mapped_file(&mut self, loc: &FileLocation) -> Option<&mut MappedFile> {
		self.file_mappings.get_mut(loc)
	}

	// TODO create_mapping (reference counting)
	// TODO remove_mapping (reference counting)
}
