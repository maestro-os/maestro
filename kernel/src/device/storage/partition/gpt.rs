/*
 * Copyright 2024 Luc Lenôtre
 *
 * This file is part of Maestro.
 *
 * Maestro is free software: you can redistribute it and/or modify it under the
 * terms of the GNU General Public License as published by the Free Software
 * Foundation, either version 3 of the License, or (at your option) any later
 * version.
 *
 * Maestro is distributed in the hope that it will be useful, but WITHOUT ANY
 * WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR
 * A PARTICULAR PURPOSE. See the GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License along with
 * Maestro. If not, see <https://www.gnu.org/licenses/>.
 */

//! The GUID Partition Table (GPT) is a standard partitions table format. It is
//! a successor of MBR.

use super::{Partition, Table};
use crate::{
	crypto::checksum::{compute_crc32, compute_crc32_lookuptable},
	device::storage::StorageInterface,
};
use core::{mem::size_of, slice};
use utils::{
	collections::vec::Vec,
	errno,
	errno::{CollectResult, EResult},
	vec,
};

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

impl Default for GPTEntry {
	fn default() -> Self {
		Self {
			partition_type: [0; 16],
			guid: [0; 16],
			start: 0,
			end: 0,
			attributes: 0,
			name: [0; 36],
		}
	}
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
	fn read_hdr_struct(storage: &mut dyn StorageInterface, lba: i64) -> EResult<Self> {
		let block_size = storage.get_block_size().get() as _;
		let blocks_count = storage.get_blocks_count();

		if size_of::<Gpt>() > block_size {
			return Err(errno!(EINVAL));
		}

		// Reading the first block
		let mut buff = vec![0; block_size]?;
		let lba = translate_lba(lba, blocks_count).ok_or_else(|| errno!(EINVAL))?;
		storage.read(buff.as_mut_slice(), lba, 1)?;

		// Valid because the header's size doesn't exceeds the size of the block
		let gpt_hdr = unsafe { &*(buff.as_ptr() as *const Gpt) };
		if !gpt_hdr.is_valid() {
			return Err(errno!(EINVAL));
		}

		Ok(gpt_hdr.clone())
	}

	/// Tells whether the header is valid.
	fn is_valid(&self) -> bool {
		if self.signature != GPT_SIGNATURE {
			return false;
		}

		// TODO Check current header LBA

		if self.entry_size == 0 {
			return false;
		}

		let mut lookup_table = [0; 256];
		compute_crc32_lookuptable(&mut lookup_table, CHECKSUM_POLYNOM);

		// Check checksum
		let mut tmp = self.clone();
		tmp.checksum = 0;
		if compute_crc32(utils::bytes::as_bytes(&tmp), &lookup_table) != self.checksum {
			return false;
		}

		// TODO check entries checksum

		true
	}

	/// Returns the list of entries in the table.
	///
	/// `storage` is the storage device interface.
	fn get_entries(&self, storage: &mut dyn StorageInterface) -> EResult<Vec<GPTEntry>> {
		let block_size = storage.get_block_size();
		let blocks_count = storage.get_blocks_count();
		let entries_start =
			translate_lba(self.entries_start, blocks_count).ok_or_else(|| errno!(EINVAL))?;
		let entries = (0..self.entries_number)
			// Read entry
			.map(|i| {
				let off = (entries_start * block_size.get()) + (i as u64 * self.entry_size as u64);
				let mut entry = GPTEntry::default();
				let slice: &mut [u8] = unsafe {
					slice::from_raw_parts_mut(
						&mut entry as *mut _ as *mut _,
						size_of::<GPTEntry>(),
					)
				};
				storage.read_bytes(slice, off)?;
				Ok(entry)
			})
			// Ignore empty entries
			.filter_map(|entry: EResult<GPTEntry>| {
				entry.map(|e| e.is_used().then_some(e)).transpose()
			})
			.map(|entry| {
				let entry = entry?;
				// Check entry correctness
				let start =
					translate_lba(entry.start, blocks_count).ok_or_else(|| errno!(EINVAL))?;
				let end = translate_lba(entry.end, blocks_count).ok_or_else(|| errno!(EINVAL))?;
				if start < end {
					Ok(entry)
				} else {
					Err(errno!(EINVAL))
				}
			})
			.collect::<EResult<CollectResult<_>>>()?
			.0?;
		Ok(entries)
	}
}

impl Table for Gpt {
	fn read(storage: &mut dyn StorageInterface) -> EResult<Option<Self>> {
		let blocks_count = storage.get_blocks_count();

		let main_hdr = match Self::read_hdr_struct(storage, 1) {
			Ok(hdr) => hdr,
			Err(e) if e == errno!(EINVAL) => return Ok(None),
			Err(e) => return Err(e),
		};
		let alternate_hdr = Self::read_hdr_struct(storage, main_hdr.alternate_hdr_lba)?;

		let main_entries = main_hdr.get_entries(storage)?;
		let alternate_entries = alternate_hdr.get_entries(storage)?;

		// Check entries correctness
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

	fn get_partitions(&self, storage: &mut dyn StorageInterface) -> EResult<Vec<Partition>> {
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
