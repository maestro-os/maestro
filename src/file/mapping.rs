//! A file mapping is a view of a file in memory, which can be modified, shared between processes,
//! etc...

use core::ptr::NonNull;
use crate::errno::Errno;
use crate::file::FileLocation;
use crate::memory::buddy;
use crate::memory;
use crate::util::container::hashmap::HashMap;

/// Structure representing a mapped page for a file.
struct Page {
	/// The pointer to the page.
	ptr: NonNull<[u8; memory::PAGE_SIZE]>,

	/// The number of references to the page.
	ref_count: u32,
}

/// A file mapped partially or totally into memory.
#[derive(Default)]
struct MappedFile {
	/// The list of mappings, ordered by offset in pages.
	pages: HashMap<usize, Page>,
}

impl MappedFile {
	/// Acquires the page at the given offset, incrementing the number of referencces to it.
	///
	/// If the page is not mapped, the function maps it.
	///
	/// `off` is the offset of the page in pages count.
	pub fn acquire_page(&mut self, off: usize) -> Result<&mut Page, Errno> {
		if !self.pages.contains_key(&off) {
			self.pages.insert(off, Page {
				ptr: NonNull::new(buddy::alloc_kernel(0)? as *mut _).unwrap(),

				ref_count: 1,
			})?;
		}

		let page = self.pages.get_mut(&off).unwrap();
		page.ref_count += 1;

		Ok(page)
	}

	/// Releases the page at the given offset, decrementing the number of references to it.
	///
	/// If the references count reaches zero, the function synchonizes the page to the disk and
	/// unmaps it.
	///
	/// `off` is the offset of the page in pages count.
	///
	/// If the page is not mapped, the function does nothing.
	pub fn release_page(&mut self, off: usize) {
		let Some(page) = self.pages.get_mut(&off) else {
			return;
		};

		page.ref_count -= 1;
		if page.ref_count == 0 {
			self.pages.remove(&off);
		}
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

	/// Returns a reference to a mapped page.
	///
	/// Arguments:
	/// - `loc` is the location to the file.
	/// - `off` is the offset of the page.
	///
	/// If the page is not mapped, the function returns `None`.
	pub fn get_page(
		&mut self,
		loc: &FileLocation,
		off: usize
	) -> Option<&mut [u8; memory::PAGE_SIZE]> {
		let file = self.mapped_files.get_mut(loc)?;
		let page = file.pages.get_mut(&off)?;

		Some(unsafe {
			page.ptr.as_mut()
		})
	}

	/// Maps the the file at the given location.
	///
	/// Arguments:
	/// - `loc` is the location to the file.
	/// - `off` is the offset of the page to map.
	pub fn map(&mut self, loc: FileLocation, _off: usize) -> Result<(), Errno> {
		let _mapped_file = match self.mapped_files.get_mut(&loc) {
			Some(f) => f,

			None => {
				self.mapped_files.insert(loc.clone(), MappedFile::default())?;
				self.mapped_files.get_mut(&loc).unwrap()
			},
		};

		// TODO increment references count on page

		Ok(())
	}

	/// Unmaps the file at the given location.
	///
	/// Arguments:
	/// - `loc` is the location to the file.
	/// - `off` is the offset of the page to unmap.
	///
	/// If the file mapping doesn't exist or the page isn't mapped, the function does nothing.
	pub fn unmap(&mut self, loc: &FileLocation, _off: usize) {
		let Some(mapped_file) = self.mapped_files.get_mut(loc) else {
			return;
		};

		// TODO decrement ref count on page

		// Remove mapping that are not referenced
		// TODO mapped_file.pages.retain(|_, p| p.ref_count <= 0);

		// If no mapping is left for the file, remove it
		if mapped_file.pages.is_empty() {
			self.mapped_files.remove(loc);
		}
	}
}
