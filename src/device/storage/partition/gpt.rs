//! The GUID Partition Table (GPT) is a standard partitions table format. It is a successor of MBR.

use core::mem::size_of;
use core::slice;
//use crate::crypto::checksum::compute_crc32;
use crate::device::storage::StorageInterface;
use crate::errno::Errno;
use crate::errno;
use crate::memory::malloc;
use crate::util::boxed::Box;
use crate::util::container::vec::Vec;
//use crate::util;
use super::Partition;
use super::Table;

/// The signature in the GPT header.
const GPT_SIGNATURE: &[u8] = b"EFI PART";
/// The polynom used in the computation of the CRC32 checksum.
const CHECKSUM_POLYNOM: u32 = 0x4c11db7;

// TODO Fix checksum check
// TODO Support negative LBA
// TODO Fix alternate table check

/// Type representing a Globally Unique IDentifier.
type GUID = [u8; 16];

/// Structure representing a GPT entry.
#[repr(C, packed)]
struct GPTEntry {
	/// The partition type's GUID.
	partition_type: GUID,
	/// The partition's GUID.
	guid: GUID,
	/// The starting LBA.
	start: u64,
	/// The ending LBA.
	end: u64,
	/// Entry's attributes.
	attributes: u64,
	/// The partition's name.
	name: [u16],
}

impl GPTEntry {
	/// Tells whether the given entry `other` equals the current entry.
	/// `entry_size` is the size of an entry.
	fn eq(&self, other: &Self, entry_size: usize) -> bool {
		if self.partition_type != other.partition_type {
			return false;
		}

		if self.guid != other.guid {
			return false;
		}

		if self.start != other.start {
			return false;
		}

		if self.end != other.end {
			return false;
		}

		if self.attributes != other.attributes {
			return false;
		}

		for i in 0..entry_size {
			if self.name[i] != other.name[i] {
				return false;
			}
		}

		true
	}
}

/// Structure representing the GPT header.
#[repr(C, packed)]
pub struct GPT {
	/// The header's signature.
	signature: [u8; 8],
	/// The header's revision.
	revision: u32,
	/// The size of the header in bytes.
	hdr_size: u32,
	/// The header's checksum.
	checksum: u32,
	/// Reserved field.
	reserved: u32,
	/// The LBA of the sector containing this header.
	hdr_lba: u64,
	/// The LBA of the sector containing the alternate header.
	alternate_hdr_lba: u64,
	/// The first usable sector.
	first_usable: u64,
	/// The last usable sector.
	last_usable: u64,
	/// The disk's GUID.
	disk_guid: GUID,
	/// The LBA of the beginning of the GUID partition entries array.
	entries_start: u64,
	/// The number of entries in the table.
	entries_number: u32,
	/// The size in bytes of each entry in the array.
	entry_size: u32,
	/// Checksum of the entries array.
	entries_checksum: u32,
}

impl Clone for GPT {
	fn clone(&self) -> Self {
		Self {
			signature: self.signature,
			revision: self.revision,
			hdr_size: self.hdr_size,
			checksum: self.checksum,
			reserved: self.reserved,
			hdr_lba: self.hdr_lba,
			alternate_hdr_lba: self.alternate_hdr_lba,
			first_usable: self.first_usable,
			last_usable: self.last_usable,
			disk_guid: self.disk_guid,
			entries_start: self.entries_start,
			entries_number: self.entries_number,
			entry_size: self.entry_size,
			entries_checksum: self.entries_checksum,
		}
	}
}

impl GPT {
	/// Reads the header structure from the given storage interface `storage` at the given LBA
	/// `lba`.
	/// If the header is invalid, the function returns an error.
	fn read_hdr_struct(storage: &mut dyn StorageInterface, lba: u64) -> Result<Self, Errno> {
		let block_size = storage.get_block_size() as usize;
		if block_size < size_of::<GPT>() {
			return Err(errno!(EINVAL));
		}

		let mut buff = malloc::Alloc::<u8>::new_default(block_size)?;

		// Reading the first block
		storage.read(buff.as_slice_mut(), lba, 1)?;

		// Valid because the header's size doesn't exceeds the size of the block
		let gpt_hdr = unsafe {
			&*(buff.as_ptr() as *const GPT)
		};
		if !gpt_hdr.is_valid() {
			return Err(errno!(EINVAL));
		}

		Ok(gpt_hdr.clone())
	}

	/// Tells whether the header is valid.
	fn is_valid(&self) -> bool {
		// Checking signature
		if self.signature != GPT_SIGNATURE {
			return false;
		}

		// Checking checksum
		/*let mut tmp = self.clone();
		tmp.checksum = 0;
		if compute_crc32(util::as_slice(&tmp), CHECKSUM_POLYNOM) != self.checksum {
			return false;
		}*/

		true
	}

	/// Returns the list of entries in the table.
	/// `storage` is the storage device interface.
	fn get_entries(&self, storage: &mut dyn StorageInterface)
		-> Result<Vec<Box<GPTEntry>>, Errno> {
		let block_size = storage.get_block_size();
		let mut entries = Vec::new();

		let mut buff = malloc::Alloc::<u8>::new_default(self.entry_size as _)?;

		for i in 0..self.entries_number {
			// Reading entry
			let off = (self.entries_start * block_size) + (i * self.entry_size) as u64;
			storage.read_bytes(buff.as_slice_mut(), off)?;

			// Inserting entry
			let entry = unsafe {
				let ptr = malloc::alloc(buff.len())? as *mut u8;
				let alloc_slice = slice::from_raw_parts_mut(ptr, buff.len());
				alloc_slice.copy_from_slice(buff.as_slice());

				Box::from_raw(alloc_slice as *mut [u8] as *mut [()] as *mut GPTEntry)
			};

			// Checking entry correctness
			if entry.end < entry.start {
				return Err(errno!(EINVAL))
			}

			entries.push(entry)?;
		}

		Ok(entries)
	}
}

impl Table for GPT {
	fn read(storage: &mut dyn StorageInterface) -> Result<Option<Self>, Errno> {
		let main_hdr = match Self::read_hdr_struct(storage, 1) {
			Ok(hdr) => hdr,
			Err(e) if e == errno!(EINVAL) => return Ok(None),
			Err(e) => return Err(e),
		};
		let alternate_hdr = Self::read_hdr_struct(storage, main_hdr.alternate_hdr_lba)?;

		let main_entries = main_hdr.get_entries(storage)?;
		let alternate_entries = alternate_hdr.get_entries(storage)?;

		// Checking entries correctness
		for (main_entry, alternate_entry) in main_entries.iter().zip(alternate_entries.iter()) {
			if !main_entry.eq(alternate_entry, main_hdr.entry_size as _) {
				return Err(errno!(EINVAL));
			}
		}

		Ok(Some(main_hdr))
	}

	fn get_type(&self) -> &'static str {
		"GPT"
	}

	fn get_partitions(&self, storage: &mut dyn StorageInterface) -> Result<Vec<Partition>, Errno> {
		let mut partitions = Vec::new();

		for e in self.get_entries(storage)? {
			partitions.push(Partition::new(e.start, e.end - e.start))?;
		}

		Ok(partitions)
	}
}
