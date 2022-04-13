//! The GUID Partition Table (GPT) is a standard partitions table format. It is a successor of MBR.

use core::mem::size_of;
use core::slice;
use crate::crypto::checksum::compute_crc32;
use crate::device::storage::StorageInterface;
use crate::errno::Errno;
use crate::errno;
use crate::memory::malloc;
use crate::util::boxed::Box;
use crate::util::container::vec::Vec;
use crate::util;
use super::Partition;
use super::Table;

/// The signature in the GPT header.
const GPT_SIGNATURE: &[u8] = b"EFI PART";
/// The polynom used in the computation of the CRC32 checksum.
const CHECKSUM_POLYNOM: u32 = 0x4c11db7;

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

/// Structure representing the GPT header.
#[derive(Clone, Copy)]
#[repr(C, packed)]
struct GPT {
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

impl GPT {
	/// Reads the header structure from the given storage interface `storage` at the given LBA
	/// `lba`.
	/// If the header is invalid, the function returns an error.
	pub fn read_hdr_struct(storage: &mut dyn StorageInterface, lba: u64) -> Result<Self, Errno> {
		let block_size = storage.get_block_size() as usize;
		if block_size < size_of::<GPT>() {
			return Err(errno!(EINVAL));
		}

		let mut buff = malloc::Alloc::<u8>::new_default(block_size)?;

		// Reading the first block
		storage.read(buff.get_slice_mut(), lba, 1)?;

		// Valid because the header's size doesn't exceeds the size of the block
		let gpt_hdr = unsafe {
			*(buff.as_ptr() as *const GPT)
		};
		if !gpt_hdr.is_valid() {
			return Err(errno!(EINVAL));
		}

		Ok(gpt_hdr)
	}

	/// Reads a GPT table from the given storage interface `storage`.
	/// If the table doesn't exist or is invalid, the function returns an error.
	pub fn read(storage: &mut dyn StorageInterface) -> Result<Self, Errno> {
		let main_hdr = Self::read_hdr_struct(storage, 1)?;
		let _alternate_hdr = Self::read_hdr_struct(storage, main_hdr.alternate_hdr_lba)?;

		let _main_entries = main_hdr.get_entries(storage)?;
		let _alternate_entries = main_hdr.get_entries(storage)?;
		// TODO Check entries correctness

		Ok(main_hdr)
	}

	/// Tells whether the header is valid.
	pub fn is_valid(&self) -> bool {
		// Checking signature
		if self.signature != GPT_SIGNATURE {
			return false;
		}

		// Checking checksum
		let mut tmp = self.clone();
		tmp.checksum = 0;
		if compute_crc32(util::as_slice(&tmp), CHECKSUM_POLYNOM) != self.checksum {
			return false;
		}

		true
	}

	/// Returns the list of entries in the table.
	/// `storage` is the storage device interface.
	pub fn get_entries(&self, storage: &mut dyn StorageInterface)
		-> Result<Vec<Box<GPTEntry>>, Errno> {
		let block_size = storage.get_block_size();
		let mut entries = Vec::new();

		let mut buff = malloc::Alloc::<u8>::new_default(self.entry_size as _)?;

		for i in 0..self.entries_number {
			// Reading entry
			let off = (self.entries_start * block_size) + (i * self.entry_size) as u64;
			storage.read_bytes(buff.get_slice_mut(), off)?;

			// Inserting entry
			unsafe {
				let ptr = malloc::alloc(buff.get_size())? as *mut u8;
				let alloc_slice = slice::from_raw_parts_mut(ptr, buff.get_size());
				alloc_slice.copy_from_slice(buff.get_slice());

				let entry = Box::from_raw(alloc_slice as *mut [u8] as *mut [()] as *mut GPTEntry);
				entries.push(entry)?;
			}
		}

		Ok(entries)
	}
}

impl Table for GPT {
	fn get_type(&self) -> &'static str {
		"GPT"
	}

	fn is_valid(&self) -> bool {
		// TODO
		todo!();
	}

	fn get_partitions(&self) -> Result<Vec<Partition>, Errno> {
		if !self.is_valid() {
			return Err(errno!(EINVAL));
		}

		// TODO
		todo!();
	}
}
