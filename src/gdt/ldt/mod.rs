//! This module handles the local descriptor table.

use crate::errno::Errno;
use crate::errno;
use crate::util::FailableClone;
use crate::util::container::vec::Vec;
use super::Entry;

extern "C" {
	/// Loads the LDT at the given pointer.
	fn ldt_load(ldt: *const LDTDescriptor);
}

/// The LDT descriptor structure.
#[repr(C, packed)]
#[derive(Clone, Copy)]
struct LDTDescriptor {
	/// The size of the LDT in bytes.
	size: u16,
	/// The linear address of the LDT.
	offset: u32,
}

/// Structure representing a LDT.
pub struct LDT {
	/// The list of entries in the LDT.
	entries: Vec<Entry>,
	/// The LDT descriptor.
	desc: LDTDescriptor,
}

impl LDT {
	/// Creates a new LDT.
	pub fn new() -> Result<Self, Errno> {
		let mut s = Self {
			entries: Vec::new(),

			desc: LDTDescriptor {
				size: 0,
				offset: 0,
			}
		};

		// Adding the null entry
		s.set(0, Entry::default())?;

		Ok(s)
	}

	/// Updates the LDT's descriptor according to the entries.
	fn update_desc(&mut self) {
		self.desc.size = (self.entries.len() * 8 - 1) as _;
		self.desc.offset = &self.entries[0] as *const _ as u32;
	}

	/// Returns the entry at index `i`.
	/// If the entry doesn't exist, the function returns None.
	pub fn get(&self, i: usize) -> Option<Entry> {
		if i < self.entries.len() {
			Some(self.entries[i])
		} else {
			None
		}
	}

	/// Sets the entry at index `i`.
	/// If the index is out of bounds, the function fails.
	pub fn set(&mut self, i: usize, entry: Entry) -> Result<(), Errno> {
		if i > 0 && i * 8 - 1 > u16::MAX as _ {
			return Err(errno::EINVAL);
		}

		self.entries.insert(i, entry)?;
		self.update_desc();

		Ok(())
	}

	/// Removes an entry from the LDT.
	/// `i` is the index of the entry to remove.
	/// If the entry doesn't exist, the function does nothing.
	pub fn remove(&mut self, i: usize) {
		self.entries.remove(i);
		self.update_desc();
	}

	/// Loads the LDT on the current thread.
	pub fn load(&self) {
		unsafe {
			ldt_load(&self.desc as *const _);
		}
	}
}

impl FailableClone for LDT {
	fn failable_clone(&self) -> Result<Self, Errno> {
		Ok(Self {
			entries: self.entries.failable_clone()?,
			desc: self.desc.clone(),
		})
	}
}
