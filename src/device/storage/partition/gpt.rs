//! The GUID Partition Table (GPT) is a standard partitions table format. It is
//! a successor of MBR.

use super::Partition;
use super::Table;
use crate::crypto::checksum::compute_crc32;
use crate::crypto::checksum::compute_crc32_lookuptable;
use crate::device::storage::StorageInterface;
use crate::errno;
use crate::errno::Errno;
use crate::memory::malloc;
use crate::util;
use crate::util::boxed::Box;
use crate::util::container::vec::Vec;
use core::mem::size_of;
use core::slice;

/// The signature in the GPT header.
const GPT_SIGNATURE: &[u8] = b"EFI PART";
/// The polynom used in the computation of the CRC32 checksum.
const CHECKSUM_POLYNOM: u32 = 0xedb88320;

// TODO Add GPT restoring from alternate table (requires user confirmation)

/// Type representing a Globally Unique IDentifier.
type Guid = [u8; 16];

/// Translates the given LBA value `lba` into a positive LBA value.
///
/// `storage_size` is the number of blocks on the storage device.
///
/// If the LBA is out of bounds of the storage device, the function returns
/// `None`.
fn translate_lba(lba: i64, storage_size: u64) -> Option<u64> {
	#[allow(clippy::collapsible_else_if)]
	if lba < 0 {
		if (-lba as u64) <= storage_size {
			Some(storage_size - (-lba as u64))
		} else {
			None
		}
	} else {
		if (lba as u64) <= storage_size {
			Some(lba as _)
		} else {
			None
		}
	}
}

/// Structure representing a GPT entry.
#[derive(Clone)]
#[repr(C, packed)]
struct GPTEntry {
	/// The partition type's GUID.
	partition_type: Guid,
	/// The partition's GUID.
	guid: Guid,
	/// The starting LBA.
	start: i64,
	/// The ending LBA.
	end: i64,
	/// Entry's attributes.
	attributes: u64,
	/// The partition's name.
	name: [u16; 36],
}

impl GPTEntry {
	/// Tells whether the given entry `other` equals the current entry.
	///
	/// Arguments:
	/// - `entry_size` is the size of an entry.
	/// - `blocks_count` is the number of blocks on the storage device.
	fn eq(&self, other: &Self, entry_size: usize, blocks_count: u64) -> bool {
		if self.partition_type != other.partition_type {
			return false;
		}

		if self.guid != other.guid {
			return false;
		}

		let start = translate_lba(self.start, blocks_count);
		let other_start = translate_lba(other.start, blocks_count);
		let end = translate_lba(self.end, blocks_count);
		let other_end = translate_lba(other.end, blocks_count);

		if start.is_none() || other_start.is_none() || end.is_none() || other_end.is_none() {
			return false;
		}
		if start.unwrap() != other_start.unwrap() || end.unwrap() != other_end.unwrap() {
			return false;
		}

		if self.attributes != other.attributes {
			return false;
		}

		let name_offset = 56; // TODO Retrieve from struct's fields
		let name_length = (entry_size - name_offset) / size_of::<u16>();
		for i in 0..name_length {
			if self.name[i] != other.name[i] {
				return false;
			}
		}

		true
	}

	/// Tells whether the entry is used.
	fn is_used(&self) -> bool {
		!self.partition_type.iter().all(|b| *b == 0)
	}
}

/// Structure representing the GPT header.
#[derive(Clone)]
#[repr(C, packed)]
pub struct Gpt {
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
	hdr_lba: i64,
	/// The LBA of the sector containing the alternate header.
	alternate_hdr_lba: i64,
	/// The first usable sector.
	first_usable: u64,
	/// The last usable sector.
	last_usable: u64,
	/// The disk's GUID.
	disk_guid: Guid,
	/// The LBA of the beginning of the GUID partition entries array.
	entries_start: i64,
	/// The number of entries in the table.
	entries_number: u32,
	/// The size in bytes of each entry in the array.
	entry_size: u32,
	/// Checksum of the entries array.
	entries_checksum: u32,
}

impl Gpt {
	/// Reads the header structure from the given storage interface `storage` at
	/// the given LBA `lba`.
	///
	/// If the header is invalid, the function returns an error.
	fn read_hdr_struct(storage: &mut dyn StorageInterface, lba: i64) -> Result<Self, Errno> {
		let block_size = storage.get_block_size() as usize;
		let blocks_count = storage.get_blocks_count();

		if size_of::<Gpt>() > block_size {
			return Err(errno!(EINVAL));
		}

		// Reading the first block
		let mut buff = malloc::Alloc::<u8>::new_default(block_size)?;
		let lba = translate_lba(lba, blocks_count).ok_or_else(|| errno!(EINVAL))?;
		storage.read(buff.as_slice_mut(), lba, 1)?;

		// Valid because the header's size doesn't exceeds the size of the block
		let gpt_hdr = unsafe { &*(buff.as_ptr() as *const Gpt) };
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

		// TODO Check current header LBA

		let mut lookup_table = [0; 256];
		compute_crc32_lookuptable(&mut lookup_table, CHECKSUM_POLYNOM);

		// Checking checksum
		let mut tmp = self.clone();
		tmp.checksum = 0;
		if compute_crc32(util::as_slice(&tmp), &lookup_table) != self.checksum {
			return false;
		}

		// TODO check entries checksum

		true
	}

	/// Returns the list of entries in the table.
	///
	/// `storage` is the storage device interface.
	fn get_entries(
		&self,
		storage: &mut dyn StorageInterface,
	) -> Result<Vec<Box<GPTEntry>>, Errno> {
		let block_size = storage.get_block_size();
		let blocks_count = storage.get_blocks_count();

		let mut buff = malloc::Alloc::<u8>::new_default(self.entry_size as _)?;

		let entries_start =
			translate_lba(self.entries_start, blocks_count).ok_or_else(|| errno!(EINVAL))?;
		let mut entries = Vec::new();

		for i in 0..self.entries_number {
			// Reading entry
			let off = (entries_start * block_size) + (i * self.entry_size) as u64;
			storage.read_bytes(buff.as_slice_mut(), off)?;

			// Inserting entry
			let entry = unsafe {
				let ptr = malloc::alloc(buff.len())? as *mut u8;
				let alloc_slice = slice::from_raw_parts_mut(ptr, buff.len());
				alloc_slice.copy_from_slice(buff.as_slice());

				Box::from_raw(alloc_slice as *mut [u8] as *mut [()] as *mut GPTEntry)
			};

			if !entry.is_used() {
				continue;
			}

			// Checking entry correctness
			let start = translate_lba(entry.start, blocks_count).ok_or_else(|| errno!(EINVAL))?;
			let end = translate_lba(entry.end, blocks_count).ok_or_else(|| errno!(EINVAL))?;
			if end < start {
				return Err(errno!(EINVAL));
			}

			entries.push(entry)?;
		}

		Ok(entries)
	}
}

impl Table for Gpt {
	fn read(storage: &mut dyn StorageInterface) -> Result<Option<Self>, Errno> {
		let blocks_count = storage.get_blocks_count();

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
			if !main_entry.eq(alternate_entry, main_hdr.entry_size as _, blocks_count) {
				return Err(errno!(EINVAL));
			}
		}

		Ok(Some(main_hdr))
	}

	fn get_type(&self) -> &'static str {
		"GPT"
	}

	fn get_partitions(&self, storage: &mut dyn StorageInterface) -> Result<Vec<Partition>, Errno> {
		let blocks_count = storage.get_blocks_count();
		let mut partitions = Vec::new();

		for e in self.get_entries(storage)? {
			let start = translate_lba(e.start, blocks_count).ok_or_else(|| errno!(EINVAL))?;
			let end = translate_lba(e.end, blocks_count).ok_or_else(|| errno!(EINVAL))?;
			// Doesn't overflow because the condition `end >= start` has already been
			// checked + 1 is required because the ending LBA is included
			let size = (end - start) + 1;

			partitions.push(Partition::new(start, size))?;
		}

		Ok(partitions)
	}
}
