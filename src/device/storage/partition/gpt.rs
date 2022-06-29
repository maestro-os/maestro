//! The GUID Partition Table (GPT) is a standard partitions table format. It is a successor of MBR.

use core::mem::size_of;
use core::slice;
//use crate::crypto::checksum::compute_crc32;
//use crate::crypto::checksum::compute_crc32_lookuptable;
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

// TODO Check checksum of entries array
// TODO Add GPT restoring from alternate table (requires user confirmation)

/// Type representing a Globally Unique IDentifier.
type GUID = [u8; 16];

/// Translates the given LBA value `lba` into a positive LBA value.
/// `storage_size` is the number of blocks on the storage device.
fn translate_lba(lba: i64, storage_size: u64) -> u64 {
	if lba < 0 {
		storage_size - (-lba as u64)
	} else {
		lba as _
	}
}

/// Structure representing a GPT entry.
#[repr(C, packed)]
struct GPTEntry {
	/// The partition type's GUID.
	partition_type: GUID,
	/// The partition's GUID.
	guid: GUID,
	/// The starting LBA.
	start: i64,
	/// The ending LBA.
	end: i64,
	/// Entry's attributes.
	attributes: u64,
	/// The partition's name.
	name: [u16],
}

impl GPTEntry {
	/// Tells whether the given entry `other` equals the current entry.
	/// `entry_size` is the size of an entry.
	/// `blocks_count` is the number of blocks on the storage device.
	fn eq(&self, other: &Self, entry_size: usize, blocks_count: u64) -> bool {
		if self.partition_type != other.partition_type {
			return false;
		}

		if self.guid != other.guid {
			return false;
		}

		if translate_lba(self.start, blocks_count) != translate_lba(other.start, blocks_count) {
			return false;
		}

		if translate_lba(self.end, blocks_count) != translate_lba(other.end, blocks_count) {
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
		!self.partition_type
			.iter()
			.all(| b | *b == 0)
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
	hdr_lba: i64,
	/// The LBA of the sector containing the alternate header.
	alternate_hdr_lba: i64,
	/// The first usable sector.
	first_usable: u64,
	/// The last usable sector.
	last_usable: u64,
	/// The disk's GUID.
	disk_guid: GUID,
	/// The LBA of the beginning of the GUID partition entries array.
	entries_start: i64,
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
	fn read_hdr_struct(storage: &mut dyn StorageInterface, lba: i64) -> Result<Self, Errno> {
		let block_size = storage.get_block_size() as usize;
		let blocks_count = storage.get_blocks_count();

		if size_of::<GPT>() > block_size {
			return Err(errno!(EINVAL));
		}

		let mut buff = malloc::Alloc::<u8>::new_default(block_size)?;

		// Reading the first block
		storage.read(buff.as_slice_mut(), translate_lba(lba, blocks_count), 1)?;

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

		// TODO Check current header LBA

		// Checking checksum
		// TODO Fix
		/*let mut tmp = self.clone();
		tmp.checksum = 0;
		let mut lookup_table = [0; 256];
		compute_crc32_lookuptable(&mut lookup_table, CHECKSUM_POLYNOM);
		if compute_crc32(util::as_slice(&tmp), &lookup_table) != self.checksum {
			return false;
		}*/

		true
	}

	/// Returns the list of entries in the table.
	/// `storage` is the storage device interface.
	fn get_entries(&self, storage: &mut dyn StorageInterface)
		-> Result<Vec<Box<GPTEntry>>, Errno> {
		let block_size = storage.get_block_size();
		let blocks_count = storage.get_blocks_count();

		let mut buff = malloc::Alloc::<u8>::new_default(self.entry_size as _)?;

		let entries_start = translate_lba(self.entries_start, blocks_count);
		let a = self.entries_start;crate::println!("----> {} {}", a, entries_start); // TODO rm
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
			let start = translate_lba(entry.start, blocks_count);
			let end = translate_lba(entry.end, blocks_count);
			let a = entry.start; let b = entry.end; crate::println!("--> {} {} {} {}", a, b, start, end); // TODO rm
			if end < start {
				return Err(errno!(EINVAL))
			}

			entries.push(entry)?;
		}

		Ok(entries)
	}
}

impl Table for GPT {
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
			let start = translate_lba(e.start, blocks_count);
			let end = translate_lba(e.end, blocks_count);
			// Doesn't overflow because the condition `end >= start` has already been checked
			// + 1 is required because the ending LBA is included
			let size = (end - start) + 1;

			partitions.push(Partition::new(start, size))?;
		}

		Ok(partitions)
	}
}
