//! The GUID Partition Table (GPT) is a standard partitions table format. It is a successor of MBR.

/// The signature in the GPT header.
const GPT_SIGNATURE: &[u8] = b"EFI PART";

/// Type representing a Globally Unique IDentifier.
type GUID = [u8; 16];

/// Structure representing a GPT entry.
#[repr(C, packed)]
pub struct GPTEntry {
	/// The partition type's GUID.
	partition_type: GUID,
	/// The partition's GUID.
	guid: GUID,
	/// The starting LBA.
	start: u64,
	/// The ending LBA.
	end: u64,
	/// TODO doc
	attributes: u64,
	/// The partition's name.
	name: [u16],
}

/// Structure representing the GPT header.
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
	/// The number of partitions in the table.
	partitions_number: u32,
	/// The size in bytes of each entry in the array.
	entry_size: u32,
	/// Checksum of the entries array.
	entries_checksum: u32,
}
